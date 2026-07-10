use super::{
    kernel_check::ensure_within_relative_error,
    math::{argmax, dot},
    matvec::f32_mul_vec,
    q4_k::q4_k_mul_vec_with_options,
    q6_k::q6_k_mul_vec_with_options,
    quantized::{q5_0_mul_vec, q8_0_argmax_mul_vec, q8_0_mul_vec},
    InferenceError, ScalarExecutionOptions,
};

mod constructors;
mod rows;

#[derive(Clone, Debug, PartialEq)]
pub struct Matrix {
    rows: usize,
    cols: usize,
    data: MatrixData,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MatrixStorageKind {
    F32,
    Q4K,
    Q5_0,
    Q6K,
    Q8_0,
}

impl MatrixStorageKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::F32 => "F32",
            Self::Q4K => "Q4_K",
            Self::Q5_0 => "Q5_0",
            Self::Q6K => "Q6_K",
            Self::Q8_0 => "Q8_0",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
enum MatrixData {
    F32(Vec<f32>),
    Q4K(Vec<u8>),
    Q5_0(Vec<u8>),
    Q6K(Vec<u8>),
    Q8_0(Vec<u8>),
}

type MatrixPairOutput = (Vec<f32>, Vec<f32>);
type MatrixTripletOutput = (Vec<f32>, Vec<f32>, Vec<f32>);

impl Matrix {
    pub fn rows(&self) -> usize {
        self.rows
    }

    pub fn cols(&self) -> usize {
        self.cols
    }

    pub fn storage_kind(&self) -> MatrixStorageKind {
        match &self.data {
            MatrixData::F32(_) => MatrixStorageKind::F32,
            MatrixData::Q4K(_) => MatrixStorageKind::Q4K,
            MatrixData::Q5_0(_) => MatrixStorageKind::Q5_0,
            MatrixData::Q6K(_) => MatrixStorageKind::Q6K,
            MatrixData::Q8_0(_) => MatrixStorageKind::Q8_0,
        }
    }

    pub fn row(&self, index: usize) -> Result<&[f32], InferenceError> {
        if index >= self.rows {
            return Err(InferenceError::new(format!(
                "matrix row {index} is out of bounds for {} rows",
                self.rows
            )));
        }
        let MatrixData::F32(data) = &self.data else {
            return Err(InferenceError::new(
                "borrowed matrix rows are only available for F32 storage",
            ));
        };

        let start = index
            .checked_mul(self.cols)
            .ok_or_else(|| InferenceError::new("matrix row offset overflow"))?;
        let end = start
            .checked_add(self.cols)
            .ok_or_else(|| InferenceError::new("matrix row end overflow"))?;
        Ok(&data[start..end])
    }

    pub fn row_values(&self, index: usize) -> Result<Vec<f32>, InferenceError> {
        rows::row_values(&self.data, self.rows, self.cols, index)
    }

    pub fn storage_bytes(&self) -> u128 {
        match &self.data {
            MatrixData::F32(values) => values.len() as u128 * std::mem::size_of::<f32>() as u128,
            MatrixData::Q4K(bytes) => bytes.len() as u128,
            MatrixData::Q5_0(bytes) => bytes.len() as u128,
            MatrixData::Q6K(bytes) => bytes.len() as u128,
            MatrixData::Q8_0(bytes) => bytes.len() as u128,
        }
    }

    pub fn mul_vec(&self, vector: &[f32]) -> Result<Vec<f32>, InferenceError> {
        self.mul_vec_with_options(vector, ScalarExecutionOptions::default())
    }

    pub fn mul_vec_with_options(
        &self,
        vector: &[f32],
        options: ScalarExecutionOptions,
    ) -> Result<Vec<f32>, InferenceError> {
        if self.cols != vector.len() {
            return Err(InferenceError::new(format!(
                "matrix columns {} do not match vector length {}",
                self.cols,
                vector.len()
            )));
        }
        ensure_vector_values_finite(vector)?;

        if let MatrixData::Q4K(data) = &self.data {
            return Ok(
                q4_k_mul_vec_with_options(data, self.rows, self.cols, vector, options)?.values,
            );
        }
        if let MatrixData::Q6K(data) = &self.data {
            return Ok(
                q6_k_mul_vec_with_options(data, self.rows, self.cols, vector, options)?.values,
            );
        }
        if let MatrixData::Q8_0(data) = &self.data {
            #[cfg(target_arch = "aarch64")]
            if options.residual_q8_activation_matvec()
                && std::arch::is_aarch64_feature_detected!("i8mm")
            {
                return super::q8_0_q8_residual_i8mm::neon_q8_0_q8_residual_i8mm_mul_vec(
                    data, self.rows, self.cols, vector,
                );
            }
            return q8_0_mul_vec(data, self.rows, self.cols, vector);
        }
        if let MatrixData::Q5_0(data) = &self.data {
            #[cfg(target_arch = "aarch64")]
            if options.residual_q8_activation_matvec()
                && std::arch::is_aarch64_feature_detected!("dotprod")
            {
                return super::q5_0_q8_residual_neon::neon_q5_0_q8_residual_mul_vec(
                    data, self.rows, self.cols, vector,
                );
            }
            return q5_0_mul_vec(data, self.rows, self.cols, vector);
        }
        if let MatrixData::F32(data) = &self.data {
            return Ok(f32_mul_vec(self.rows, self.cols, data, vector)?.into_values());
        }

        let mut output = Vec::with_capacity(self.rows);
        for row_index in 0..self.rows {
            let row = self.row_values(row_index)?;
            output.push(dot(&row, vector)?);
        }
        Ok(output)
    }

    /// Uses a paired Q5_0 kernel when both matrices have the same shape and
    /// execution policy. `None` tells callers to use independent dispatch.
    pub(in crate::scalar) fn mul_vec_pair_with_options(
        &self,
        other: &Self,
        vector: &[f32],
        left_options: ScalarExecutionOptions,
        right_options: ScalarExecutionOptions,
    ) -> Result<Option<MatrixPairOutput>, InferenceError> {
        #[cfg(not(target_arch = "aarch64"))]
        {
            let _ = (other, vector, left_options, right_options);
            return Ok(None);
        }
        #[cfg(target_arch = "aarch64")]
        {
            if self.rows != other.rows || self.cols != other.cols {
                return Ok(None);
            }
            if self.cols != vector.len() {
                return Err(InferenceError::new(format!(
                    "matrix columns {} do not match vector length {}",
                    self.cols,
                    vector.len()
                )));
            }
            let (MatrixData::Q5_0(left), MatrixData::Q5_0(right)) = (&self.data, &other.data)
            else {
                return Ok(None);
            };
            ensure_vector_values_finite(vector)?;
            let left_residual = left_options.residual_q8_activation_matvec();
            let right_residual = right_options.residual_q8_activation_matvec();
            if left_residual != right_residual {
                return Ok(None);
            }
            if left_residual && std::arch::is_aarch64_feature_detected!("dotprod") {
                return super::q5_0_q8_residual_neon::neon_q5_0_q8_residual_mul_vec_pair(
                    left, right, self.rows, self.cols, vector,
                )
                .map(Some);
            }
            super::q5_0::q5_0_mul_vec_pair(left, right, self.rows, self.cols, vector).map(Some)
        }
    }

    /// Computes attention Q/K/V together while sharing one residual-Q8
    /// activation across candidate Q5_0 projections. Returns `None` when
    /// fewer than two projections can share that work.
    pub(in crate::scalar) fn mul_vec_qkv_with_options(
        &self,
        key: &Self,
        value: &Self,
        vector: &[f32],
        query_options: ScalarExecutionOptions,
        key_options: ScalarExecutionOptions,
        value_options: ScalarExecutionOptions,
    ) -> Result<Option<MatrixTripletOutput>, InferenceError> {
        #[cfg(not(target_arch = "aarch64"))]
        {
            let _ = (
                key,
                value,
                vector,
                query_options,
                key_options,
                value_options,
            );
            return Ok(None);
        }
        #[cfg(target_arch = "aarch64")]
        {
            if self.cols != vector.len()
                || key.cols != vector.len()
                || value.cols != vector.len()
                || !std::arch::is_aarch64_feature_detected!("dotprod")
            {
                return Ok(None);
            }
            let candidates = [
                self.is_q5_residual_candidate(query_options),
                key.is_q5_residual_candidate(key_options),
                value.is_q5_residual_candidate(value_options),
            ];
            if candidates
                .into_iter()
                .filter(|candidate| *candidate)
                .count()
                < 2
            {
                return Ok(None);
            }

            ensure_vector_values_finite(vector)?;
            let activation =
                super::q8_residual_activation::BlockQ8Residual::quantize_blocks(vector)?;
            let (query, (key, value)) = rayon::join(
                || self.mul_vec_with_shared_q5_residual(vector, query_options, &activation),
                || {
                    rayon::join(
                        || key.mul_vec_with_shared_q5_residual(vector, key_options, &activation),
                        || {
                            value.mul_vec_with_shared_q5_residual(
                                vector,
                                value_options,
                                &activation,
                            )
                        },
                    )
                },
            );
            Ok(Some((query?, key?, value?)))
        }
    }

    #[cfg(target_arch = "aarch64")]
    fn is_q5_residual_candidate(&self, options: ScalarExecutionOptions) -> bool {
        options.residual_q8_activation_matvec() && matches!(&self.data, MatrixData::Q5_0(_))
    }

    #[cfg(target_arch = "aarch64")]
    fn mul_vec_with_shared_q5_residual(
        &self,
        vector: &[f32],
        options: ScalarExecutionOptions,
        activation: &[super::q8_residual_activation::BlockQ8Residual],
    ) -> Result<Vec<f32>, InferenceError> {
        if options.residual_q8_activation_matvec() {
            if let MatrixData::Q5_0(data) = &self.data {
                return super::q5_0_q8_residual_neon::neon_q5_0_q8_residual_mul_vec_prequantized(
                    data, self.rows, self.cols, activation,
                );
            }
        }
        self.mul_vec_with_options(vector, options)
    }

    /// Multiplies several activation vectors against this matrix in one
    /// pass. Storage kinds with a batched kernel stream each weight row
    /// once for the whole batch; the rest fall back to per-vector matvecs.
    /// Every stream's output is bit-identical to `mul_vec` on that vector.
    pub fn mul_vec_batch(&self, vectors: &[&[f32]]) -> Result<Vec<Vec<f32>>, InferenceError> {
        for vector in vectors {
            if self.cols != vector.len() {
                return Err(InferenceError::new(format!(
                    "matrix columns {} do not match vector length {}",
                    self.cols,
                    vector.len()
                )));
            }
            ensure_vector_values_finite(vector)?;
        }

        if let MatrixData::Q5_0(data) = &self.data {
            return super::q5_0::q5_0_mul_vec_batch(data, self.rows, self.cols, vectors);
        }
        if let MatrixData::Q8_0(data) = &self.data {
            return super::q8_0::q8_0_mul_vec_batch(data, self.rows, self.cols, vectors);
        }
        if let MatrixData::Q6K(data) = &self.data {
            return super::q6_k::q6_k_mul_vec_batch(data, self.rows, self.cols, vectors);
        }
        if let MatrixData::Q4K(data) = &self.data {
            return super::q4_k::q4_k_mul_vec_batch(data, self.rows, self.cols, vectors);
        }

        vectors.iter().map(|vector| self.mul_vec(vector)).collect()
    }

    /// Greedy argmax for several activation vectors in one weight pass
    /// where the storage kind supports it; per-stream results equal
    /// `argmax_mul_vec` on that vector.
    pub fn argmax_mul_vec_batch(&self, vectors: &[&[f32]]) -> Result<Vec<usize>, InferenceError> {
        for vector in vectors {
            if self.cols != vector.len() {
                return Err(InferenceError::new(format!(
                    "matrix columns {} do not match vector length {}",
                    self.cols,
                    vector.len()
                )));
            }
            ensure_vector_values_finite(vector)?;
        }
        if self.rows == 0 {
            return Err(InferenceError::new("argmax input must not be empty"));
        }

        if let MatrixData::Q8_0(data) = &self.data {
            return super::q8_0::q8_0_argmax_mul_vec_batch(data, self.rows, self.cols, vectors);
        }

        vectors
            .iter()
            .map(|vector| self.argmax_mul_vec(vector))
            .collect()
    }

    pub fn argmax_mul_vec(&self, vector: &[f32]) -> Result<usize, InferenceError> {
        self.argmax_mul_vec_with_options(vector, ScalarExecutionOptions::default())
    }

    pub fn argmax_mul_vec_with_options(
        &self,
        vector: &[f32],
        options: ScalarExecutionOptions,
    ) -> Result<usize, InferenceError> {
        if self.cols != vector.len() {
            return Err(InferenceError::new(format!(
                "matrix columns {} do not match vector length {}",
                self.cols,
                vector.len()
            )));
        }
        ensure_vector_values_finite(vector)?;
        if self.rows == 0 {
            return Err(InferenceError::new("argmax input must not be empty"));
        }

        #[cfg(target_arch = "aarch64")]
        if matches!(&self.data, MatrixData::Q6K(_))
            && (options.q8_k_activation_matvec() || options.residual_q8_activation_matvec())
        {
            return argmax(&self.mul_vec_with_options(vector, options)?);
        }
        if let MatrixData::Q6K(data) = &self.data {
            return super::q6_k::q6_k_argmax_mul_vec(data, self.rows, self.cols, vector);
        }
        if let MatrixData::Q8_0(data) = &self.data {
            #[cfg(target_arch = "aarch64")]
            if options.residual_q8_activation_matvec()
                && std::arch::is_aarch64_feature_detected!("i8mm")
            {
                return super::q8_0_q8_residual_i8mm::neon_q8_0_q8_residual_i8mm_argmax(
                    data, self.rows, self.cols, vector,
                );
            }
            return q8_0_argmax_mul_vec(data, self.rows, self.cols, vector);
        }

        argmax(&self.mul_vec_with_options(vector, options)?)
    }

    pub fn mul_vec_checked_against_reference(
        &self,
        vector: &[f32],
        relative_error_tolerance: f32,
    ) -> Result<Vec<f32>, InferenceError> {
        let output = self.mul_vec(vector)?;
        let reference = self.mul_vec_scalar_reference(vector)?;
        ensure_within_relative_error(&output, &reference, relative_error_tolerance)?;
        Ok(output)
    }

    fn mul_vec_scalar_reference(&self, vector: &[f32]) -> Result<Vec<f32>, InferenceError> {
        if self.cols != vector.len() {
            return Err(InferenceError::new(format!(
                "matrix columns {} do not match vector length {}",
                self.cols,
                vector.len()
            )));
        }
        ensure_vector_values_finite(vector)?;

        let mut output = Vec::with_capacity(self.rows);
        for row_index in 0..self.rows {
            let row = self.row_values(row_index)?;
            output.push(dot(&row, vector)?);
        }
        Ok(output)
    }
}

fn ensure_vector_values_finite(vector: &[f32]) -> Result<(), InferenceError> {
    if vector.iter().any(|value| !value.is_finite()) {
        return Err(InferenceError::new("matrix vector values must be finite"));
    }
    Ok(())
}
