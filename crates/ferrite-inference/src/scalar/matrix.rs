use super::{
    dense16::{bf16_mul_vec_with_options, f16_mul_vec_with_options},
    kernel_check::ensure_within_relative_error,
    math::{argmax, dot},
    matvec::f32_mul_vec_with_options,
    q4_k::q4_k_mul_vec_with_options,
    q5_0::q5_0_mul_vec_with_options,
    q5_k::q5_k_mul_vec_with_options,
    q6_k::q6_k_mul_vec_with_options,
    q8_0::{q8_0_argmax_mul_vec_with_options, q8_0_mul_vec_with_options},
    InferenceError, ScalarExecutionOptions,
};
use ferrite_model::model_file::MappedModelFile;
use std::{ops::Deref, ops::Range};

mod constructors;
mod rows;

#[derive(Clone, Debug, PartialEq)]
/// A validated row-major matrix using dense or supported GGML quantized storage.
pub struct Matrix {
    rows: usize,
    cols: usize,
    data: MatrixData,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
/// The physical storage representation of a [`Matrix`].
pub enum MatrixStorageKind {
    /// Row-major 32-bit floating-point values.
    F32,
    /// Row-major IEEE 16-bit floating-point values.
    F16,
    /// Row-major brain floating-point values.
    BF16,
    /// GGML `Q4_K` quantized blocks.
    Q4K,
    /// GGML `Q5_0` quantized blocks.
    Q5_0,
    /// GGML `Q5_K` quantized blocks.
    Q5K,
    /// GGML `Q6_K` quantized blocks.
    Q6K,
    /// GGML `Q8_0` quantized blocks.
    Q8_0,
}

impl MatrixStorageKind {
    /// Returns the stable GGML-style label used in diagnostics and profiles.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::F32 => "F32",
            Self::F16 => "F16",
            Self::BF16 => "BF16",
            Self::Q4K => "Q4_K",
            Self::Q5_0 => "Q5_0",
            Self::Q5K => "Q5_K",
            Self::Q6K => "Q6_K",
            Self::Q8_0 => "Q8_0",
        }
    }
}

#[derive(Clone, Debug)]
enum MatrixData {
    F32(Vec<f32>),
    F16(MatrixBytes),
    BF16(MatrixBytes),
    Q4K(MatrixBytes),
    Q5_0(MatrixBytes),
    Q5K(MatrixBytes),
    Q6K(MatrixBytes),
    Q8_0(MatrixBytes),
}

impl PartialEq for MatrixData {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::F32(left), Self::F32(right)) => left == right,
            (Self::F16(left), Self::F16(right))
            | (Self::BF16(left), Self::BF16(right))
            | (Self::Q4K(left), Self::Q4K(right))
            | (Self::Q5_0(left), Self::Q5_0(right))
            | (Self::Q5K(left), Self::Q5K(right))
            | (Self::Q6K(left), Self::Q6K(right))
            | (Self::Q8_0(left), Self::Q8_0(right)) => left == right,
            _ => false,
        }
    }
}

#[derive(Clone, Debug)]
enum MatrixBytes {
    Owned(Vec<u8>),
    Mapped {
        file: MappedModelFile,
        range: Range<usize>,
    },
}

impl MatrixBytes {
    fn mapped(file: MappedModelFile, range: Range<usize>) -> Result<Self, InferenceError> {
        if file.as_bytes().get(range.clone()).is_none() {
            return Err(InferenceError::new(format!(
                "matrix byte range {range:?} is invalid for {} mapped bytes",
                file.as_bytes().len()
            )));
        }
        Ok(Self::Mapped { file, range })
    }

    fn as_slice(&self) -> &[u8] {
        match self {
            Self::Owned(bytes) => bytes,
            Self::Mapped { file, range } => &file.as_bytes()[range.clone()],
        }
    }

    fn mapped_file_bytes(&self) -> usize {
        match self {
            Self::Owned(_) => 0,
            Self::Mapped { file, .. } => file.as_bytes().len(),
        }
    }

    fn slice(&self, relative: Range<usize>) -> Result<Self, InferenceError> {
        let selected = self.as_slice().get(relative.clone()).ok_or_else(|| {
            InferenceError::new(format!(
                "matrix subrange {relative:?} is out of bounds for {} bytes",
                self.as_slice().len()
            ))
        })?;
        match self {
            Self::Owned(_) => Ok(Self::Owned(selected.to_vec())),
            Self::Mapped { file, range } => {
                let start = range
                    .start
                    .checked_add(relative.start)
                    .ok_or_else(|| InferenceError::new("mapped matrix subrange start overflow"))?;
                let end = range
                    .start
                    .checked_add(relative.end)
                    .ok_or_else(|| InferenceError::new("mapped matrix subrange end overflow"))?;
                Self::mapped(file.clone(), start..end)
            }
        }
    }
}

impl Deref for MatrixBytes {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        self.as_slice()
    }
}

impl PartialEq for MatrixBytes {
    fn eq(&self, other: &Self) -> bool {
        self.as_slice() == other.as_slice()
    }
}

type MatrixPairOutput = (Vec<f32>, Vec<f32>);
type MatrixTripletOutput = (Vec<f32>, Vec<f32>, Vec<f32>);

impl Matrix {
    /// Returns the matrix row count.
    pub fn rows(&self) -> usize {
        self.rows
    }

    /// Returns the matrix column count.
    pub fn cols(&self) -> usize {
        self.cols
    }

    /// Returns the matrix's physical storage representation.
    pub fn storage_kind(&self) -> MatrixStorageKind {
        match &self.data {
            MatrixData::F32(_) => MatrixStorageKind::F32,
            MatrixData::F16(_) => MatrixStorageKind::F16,
            MatrixData::BF16(_) => MatrixStorageKind::BF16,
            MatrixData::Q4K(_) => MatrixStorageKind::Q4K,
            MatrixData::Q5_0(_) => MatrixStorageKind::Q5_0,
            MatrixData::Q5K(_) => MatrixStorageKind::Q5K,
            MatrixData::Q6K(_) => MatrixStorageKind::Q6K,
            MatrixData::Q8_0(_) => MatrixStorageKind::Q8_0,
        }
    }

    /// Borrows one row from an F32 matrix.
    ///
    /// # Errors
    ///
    /// Returns an error when `index` is out of range or the matrix uses a
    /// quantized representation that cannot expose borrowed F32 values.
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

    /// Decodes and returns one matrix row as F32 values.
    ///
    /// # Errors
    ///
    /// Returns an error when `index` is out of range or quantized data fails
    /// structural or numeric validation.
    pub fn row_values(&self, index: usize) -> Result<Vec<f32>, InferenceError> {
        rows::row_values(&self.data, self.rows, self.cols, index)
    }

    /// Returns a matrix containing a contiguous range of rows.
    ///
    /// Mapped dense-16 and quantized matrices retain a read-only view of the
    /// selected byte range. Owned matrices copy only the selected rows.
    ///
    /// # Errors
    ///
    /// Returns an error for an empty or out-of-bounds range, arithmetic
    /// overflow, or a quantized shape whose rows do not end on block
    /// boundaries.
    pub fn row_range(&self, range: Range<usize>) -> Result<Self, InferenceError> {
        if range.start >= range.end || range.end > self.rows {
            return Err(InferenceError::new(format!(
                "matrix row range {range:?} is invalid for {} rows",
                self.rows
            )));
        }
        let selected_rows = range.end - range.start;
        let data = match &self.data {
            MatrixData::F32(values) => {
                let start = range
                    .start
                    .checked_mul(self.cols)
                    .ok_or_else(|| InferenceError::new("F32 matrix row range start overflow"))?;
                let end = range
                    .end
                    .checked_mul(self.cols)
                    .ok_or_else(|| InferenceError::new("F32 matrix row range end overflow"))?;
                MatrixData::F32(values[start..end].to_vec())
            }
            MatrixData::F16(bytes) => MatrixData::F16(slice_matrix_rows(
                bytes,
                range,
                dense16_row_bytes(self.cols)?,
            )?),
            MatrixData::BF16(bytes) => MatrixData::BF16(slice_matrix_rows(
                bytes,
                range,
                dense16_row_bytes(self.cols)?,
            )?),
            MatrixData::Q4K(bytes) => MatrixData::Q4K(slice_matrix_rows(
                bytes,
                range,
                super::q4_k::q4_k_storage_bytes(self.cols)?,
            )?),
            MatrixData::Q5_0(bytes) => MatrixData::Q5_0(slice_matrix_rows(
                bytes,
                range,
                super::q5_0::q5_0_row_bytes(self.cols)?,
            )?),
            MatrixData::Q5K(bytes) => MatrixData::Q5K(slice_matrix_rows(
                bytes,
                range,
                super::q5_k::q5_k_storage_bytes(self.cols)?,
            )?),
            MatrixData::Q6K(bytes) => MatrixData::Q6K(slice_matrix_rows(
                bytes,
                range,
                super::q6_k::q6_k_storage_bytes(self.cols)?,
            )?),
            MatrixData::Q8_0(bytes) => MatrixData::Q8_0(slice_matrix_rows(
                bytes,
                range,
                super::q8_0::q8_0_row_bytes(self.cols)?,
            )?),
        };
        Ok(Self {
            rows: selected_rows,
            cols: self.cols,
            data,
        })
    }

    /// Returns the byte count of the matrix's physical tensor storage.
    ///
    /// The storage may be owned heap memory or a range retained from a shared
    /// read-only GGUF mapping.
    pub fn storage_bytes(&self) -> u128 {
        match &self.data {
            MatrixData::F32(values) => values.len() as u128 * std::mem::size_of::<f32>() as u128,
            MatrixData::F16(bytes) | MatrixData::BF16(bytes) => bytes.len() as u128,
            MatrixData::Q4K(bytes) => bytes.len() as u128,
            MatrixData::Q5_0(bytes) => bytes.len() as u128,
            MatrixData::Q5K(bytes) => bytes.len() as u128,
            MatrixData::Q6K(bytes) => bytes.len() as u128,
            MatrixData::Q8_0(bytes) => bytes.len() as u128,
        }
    }

    pub(in crate::scalar) fn mapped_file_bytes(&self) -> usize {
        match &self.data {
            MatrixData::F32(_) => 0,
            MatrixData::F16(bytes)
            | MatrixData::BF16(bytes)
            | MatrixData::Q4K(bytes)
            | MatrixData::Q5_0(bytes)
            | MatrixData::Q5K(bytes)
            | MatrixData::Q6K(bytes)
            | MatrixData::Q8_0(bytes) => bytes.mapped_file_bytes(),
        }
    }

    /// Multiplies this matrix by one F32 activation vector.
    ///
    /// # Errors
    ///
    /// Returns an error for a length mismatch, non-finite input, malformed
    /// storage, arithmetic overflow, or non-finite kernel result.
    pub fn mul_vec(&self, vector: &[f32]) -> Result<Vec<f32>, InferenceError> {
        self.mul_vec_with_options(vector, ScalarExecutionOptions::default())
    }

    /// Multiplies this matrix by one vector with an explicit kernel policy.
    ///
    /// # Errors
    ///
    /// Returns an error for a length mismatch, non-finite input, malformed
    /// storage, arithmetic overflow, or non-finite kernel result.
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
        if let MatrixData::Q5K(data) = &self.data {
            return q5_k_mul_vec_with_options(data, self.rows, self.cols, vector, options);
        }
        if let MatrixData::Q8_0(data) = &self.data {
            #[cfg(target_arch = "aarch64")]
            if options.residual_q8_activation_matvec() && options.kernel_dispatch().i8mm() {
                return super::q8_0_q8_residual_i8mm::neon_q8_0_q8_residual_i8mm_mul_vec(
                    data, self.rows, self.cols, vector,
                );
            }
            return q8_0_mul_vec_with_options(data, self.rows, self.cols, vector, options);
        }
        if let MatrixData::Q5_0(data) = &self.data {
            #[cfg(target_arch = "aarch64")]
            if options.residual_q8_activation_matvec() && options.kernel_dispatch().dotprod() {
                return super::q5_0_q8_residual_neon::neon_q5_0_q8_residual_mul_vec(
                    data, self.rows, self.cols, vector,
                );
            }
            return q5_0_mul_vec_with_options(data, self.rows, self.cols, vector, options);
        }
        if let MatrixData::F32(data) = &self.data {
            return Ok(
                f32_mul_vec_with_options(self.rows, self.cols, data, vector, options)?
                    .into_values(),
            );
        }
        if let MatrixData::F16(data) = &self.data {
            return f16_mul_vec_with_options(data, self.rows, self.cols, vector, options);
        }
        if let MatrixData::BF16(data) = &self.data {
            return bf16_mul_vec_with_options(data, self.rows, self.cols, vector, options);
        }

        let mut output = Vec::with_capacity(self.rows);
        for row_index in 0..self.rows {
            let row = self.row_values(row_index)?;
            output.push(dot(&row, vector)?);
        }
        Ok(output)
    }

    /// Uses a paired `Q5_0` kernel when both matrices have the same shape and
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
            Ok(None)
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
            if left_residual && left_options.kernel_dispatch().dotprod() {
                return super::q5_0_q8_residual_neon::neon_q5_0_q8_residual_mul_vec_pair(
                    left, right, self.rows, self.cols, vector,
                )
                .map(Some);
            }
            super::q5_0::q5_0_mul_vec_pair_with_options(
                left,
                right,
                self.rows,
                self.cols,
                vector,
                left_options,
            )
            .map(Some)
        }
    }

    /// Computes attention Q/K/V together while sharing one residual-Q8
    /// activation across candidate `Q5_0` projections. Returns `None` when
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
            Ok(None)
        }
        #[cfg(target_arch = "aarch64")]
        {
            if self.cols != vector.len()
                || key.cols != vector.len()
                || value.cols != vector.len()
                || !query_options.kernel_dispatch().dotprod()
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
        options.residual_q8_activation_matvec()
            && options.kernel_dispatch().dotprod()
            && matches!(&self.data, MatrixData::Q5_0(_))
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
    /// Every stream's output is bit-identical to [`Self::mul_vec`] on that
    /// vector.
    ///
    /// # Errors
    ///
    /// Returns an error for a vector length mismatch, non-finite input,
    /// malformed matrix storage, arithmetic overflow, or non-finite output.
    pub fn mul_vec_batch(&self, vectors: &[&[f32]]) -> Result<Vec<Vec<f32>>, InferenceError> {
        self.mul_vec_batch_with_options(vectors, ScalarExecutionOptions::default())
    }

    pub(in crate::scalar) fn mul_vec_batch_with_options(
        &self,
        vectors: &[&[f32]],
        options: ScalarExecutionOptions,
    ) -> Result<Vec<Vec<f32>>, InferenceError> {
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
            return super::q5_0::q5_0_mul_vec_batch_with_options(
                data, self.rows, self.cols, vectors, options,
            );
        }
        if let MatrixData::Q8_0(data) = &self.data {
            return super::q8_0::q8_0_mul_vec_batch_with_options(
                data, self.rows, self.cols, vectors, options,
            );
        }
        if let MatrixData::Q6K(data) = &self.data {
            return super::q6_k::q6_k_mul_vec_batch_with_options(
                data, self.rows, self.cols, vectors, options,
            );
        }
        if let MatrixData::Q4K(data) = &self.data {
            return super::q4_k::q4_k_mul_vec_batch_with_options(
                data, self.rows, self.cols, vectors, options,
            );
        }
        if let MatrixData::Q5K(data) = &self.data {
            return super::q5_k::q5_k_mul_vec_batch_with_options(
                data, self.rows, self.cols, vectors, options,
            );
        }

        vectors
            .iter()
            .map(|vector| self.mul_vec_with_options(vector, options))
            .collect()
    }

    /// Greedy argmax for several activation vectors in one weight pass
    /// where the storage kind supports it; per-stream results equal
    /// [`Self::argmax_mul_vec`] on that vector.
    ///
    /// # Errors
    ///
    /// Returns an error for an empty matrix, vector length mismatch,
    /// non-finite input, malformed storage, or non-finite output.
    pub fn argmax_mul_vec_batch(&self, vectors: &[&[f32]]) -> Result<Vec<usize>, InferenceError> {
        self.argmax_mul_vec_batch_with_options(vectors, ScalarExecutionOptions::default())
    }

    pub(in crate::scalar) fn argmax_mul_vec_batch_with_options(
        &self,
        vectors: &[&[f32]],
        options: ScalarExecutionOptions,
    ) -> Result<Vec<usize>, InferenceError> {
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
            return super::q8_0::q8_0_argmax_mul_vec_batch_with_options(
                data, self.rows, self.cols, vectors, options,
            );
        }

        vectors
            .iter()
            .map(|vector| self.argmax_mul_vec_with_options(vector, options))
            .collect()
    }

    /// Returns the highest-output row for one activation vector.
    ///
    /// # Errors
    ///
    /// Returns an error for an empty matrix, a length mismatch, non-finite
    /// input, malformed storage, or a kernel failure.
    pub fn argmax_mul_vec(&self, vector: &[f32]) -> Result<usize, InferenceError> {
        self.argmax_mul_vec_with_options(vector, ScalarExecutionOptions::default())
    }

    /// Returns the highest-output row using an explicit kernel policy.
    ///
    /// # Errors
    ///
    /// Returns an error for an empty matrix, a length mismatch, non-finite
    /// input, malformed storage, or a kernel failure.
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
            return super::q6_k::q6_k_argmax_mul_vec_with_options(
                data, self.rows, self.cols, vector, options,
            );
        }
        if let MatrixData::Q8_0(data) = &self.data {
            #[cfg(target_arch = "aarch64")]
            if options.residual_q8_activation_matvec() && options.kernel_dispatch().i8mm() {
                return super::q8_0_q8_residual_i8mm::neon_q8_0_q8_residual_i8mm_argmax(
                    data, self.rows, self.cols, vector,
                );
            }
            return q8_0_argmax_mul_vec_with_options(data, self.rows, self.cols, vector, options);
        }

        argmax(&self.mul_vec_with_options(vector, options)?)
    }

    /// Runs normal dispatch and verifies it against decoded scalar row dots.
    ///
    /// # Errors
    ///
    /// Returns an error for invalid inputs, a kernel failure, or an output that
    /// exceeds `relative_error_tolerance` from the reference result.
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

fn slice_matrix_rows(
    bytes: &MatrixBytes,
    range: Range<usize>,
    row_bytes: usize,
) -> Result<MatrixBytes, InferenceError> {
    let start = range
        .start
        .checked_mul(row_bytes)
        .ok_or_else(|| InferenceError::new("matrix row range start overflow"))?;
    let end = range
        .end
        .checked_mul(row_bytes)
        .ok_or_else(|| InferenceError::new("matrix row range end overflow"))?;
    bytes.slice(start..end)
}

fn dense16_row_bytes(cols: usize) -> Result<usize, InferenceError> {
    cols.checked_mul(2)
        .ok_or_else(|| InferenceError::new("dense-16 matrix row byte length overflow"))
}
