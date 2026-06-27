pub(crate) const VALUE_STRING: u32 = 8;
pub(crate) const VALUE_ARRAY: u32 = 9;
pub(crate) const VALUE_UINT64: u32 = 10;
pub(crate) const GGML_TYPE_F32: u32 = 0;
pub(crate) const GGML_TYPE_F16: u32 = 1;
pub(crate) const GGML_TYPE_Q5_0: u32 = 6;
pub(crate) const GGML_TYPE_Q8_0: u32 = 8;
pub(crate) const GGML_TYPE_Q4_K: u32 = 12;
pub(crate) const GGML_TYPE_Q6_K: u32 = 14;
pub(crate) const GGML_TYPE_BF16: u32 = 30;

pub(crate) struct F32TensorFixture {
    pub(crate) name: &'static str,
    pub(crate) dimensions: Vec<u64>,
    pub(crate) values: Vec<f32>,
    pub(crate) offset: u64,
}

pub(crate) struct TypedTensorFixture {
    pub(crate) name: &'static str,
    pub(crate) dimensions: Vec<u64>,
    pub(crate) values: Vec<f32>,
    pub(crate) tensor_type: u32,
    pub(crate) offset: u64,
}

pub(crate) fn push_u32(bytes: &mut Vec<u8>, value: u32) {
    bytes.extend_from_slice(&value.to_le_bytes());
}

pub(crate) fn push_u64(bytes: &mut Vec<u8>, value: u64) {
    bytes.extend_from_slice(&value.to_le_bytes());
}

pub(crate) fn push_kv_string(bytes: &mut Vec<u8>, key: &str, value: &str) {
    push_string(bytes, key);
    push_u32(bytes, VALUE_STRING);
    push_string(bytes, value);
}

pub(crate) fn push_kv_u64(bytes: &mut Vec<u8>, key: &str, value: u64) {
    push_string(bytes, key);
    push_u32(bytes, VALUE_UINT64);
    push_u64(bytes, value);
}

pub(crate) fn push_kv_string_array(bytes: &mut Vec<u8>, key: &str, values: &[&str]) {
    push_string(bytes, key);
    push_u32(bytes, VALUE_ARRAY);
    push_u32(bytes, VALUE_STRING);
    push_u64(bytes, values.len() as u64);
    for value in values {
        push_string(bytes, value);
    }
}

pub(crate) fn push_tensor_info_with_type(
    bytes: &mut Vec<u8>,
    tensor: &F32TensorFixture,
    tensor_type: u32,
) {
    push_string(bytes, tensor.name);
    push_u32(bytes, tensor.dimensions.len() as u32);
    for dimension in &tensor.dimensions {
        push_u64(bytes, *dimension);
    }
    push_u32(bytes, tensor_type);
    push_u64(bytes, tensor.offset);
}

pub(crate) fn push_typed_tensor_info(bytes: &mut Vec<u8>, tensor: &TypedTensorFixture) {
    push_string(bytes, tensor.name);
    push_u32(bytes, tensor.dimensions.len() as u32);
    for dimension in &tensor.dimensions {
        push_u64(bytes, *dimension);
    }
    push_u32(bytes, tensor.tensor_type);
    push_u64(bytes, tensor.offset);
}

pub(crate) fn align_value(value: u64, alignment: u64) -> u64 {
    let remainder = value % alignment;
    if remainder == 0 {
        value
    } else {
        value + alignment - remainder
    }
}

pub(crate) fn align_len(bytes: &mut Vec<u8>, alignment: usize) {
    let padding = (alignment - (bytes.len() % alignment)) % alignment;
    bytes.resize(bytes.len() + padding, 0);
}

pub(crate) fn q8_storage_bytes(value_count: usize) -> u64 {
    (value_count / 32 * 34) as u64
}

pub(crate) fn typed_storage_bytes(tensor: &TypedTensorFixture) -> u64 {
    if tensor.tensor_type == GGML_TYPE_Q4_K {
        q4_k_storage_bytes(tensor.values.len())
    } else if tensor.tensor_type == GGML_TYPE_Q6_K {
        q6_k_storage_bytes(tensor.values.len())
    } else if tensor.tensor_type == GGML_TYPE_Q5_0 {
        q5_0_storage_bytes(tensor.values.len())
    } else {
        (tensor.values.len() * 4) as u64
    }
}

pub(crate) fn push_typed_tensor_values(bytes: &mut Vec<u8>, tensor: &TypedTensorFixture) {
    if tensor.tensor_type == GGML_TYPE_Q4_K {
        push_q4_k_values(bytes, &tensor.values);
    } else if tensor.tensor_type == GGML_TYPE_Q6_K {
        push_q6_k_values(bytes, &tensor.values);
    } else if tensor.tensor_type == GGML_TYPE_Q5_0 {
        push_q5_0_values(bytes, &tensor.values);
    } else {
        for value in &tensor.values {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
    }
}

pub(crate) fn push_q8_0_values(bytes: &mut Vec<u8>, values: &[f32]) {
    for block in values.chunks_exact(32) {
        bytes.extend_from_slice(&f32_to_f16_bits(1.0).to_le_bytes());
        for value in block {
            bytes.push(value.round() as i8 as u8);
        }
    }
}

pub(crate) fn f32_to_f16_bits(value: f32) -> u16 {
    let bits = value.to_bits();
    let sign = ((bits >> 16) & 0x8000) as u16;
    let exponent = ((bits >> 23) & 0xff) as i32 - 127 + 15;
    let mantissa = bits & 0x7fffff;

    if exponent <= 0 {
        return sign;
    }
    if exponent >= 0x1f {
        return sign | 0x7c00;
    }

    sign | ((exponent as u16) << 10) | ((mantissa >> 13) as u16)
}

pub(crate) fn f32_to_bf16_bits(value: f32) -> u16 {
    (value.to_bits() >> 16) as u16
}

fn push_string(bytes: &mut Vec<u8>, value: &str) {
    push_u64(bytes, value.len() as u64);
    bytes.extend_from_slice(value.as_bytes());
}

fn q4_k_storage_bytes(value_count: usize) -> u64 {
    (value_count / 256 * 144) as u64
}

fn q6_k_storage_bytes(value_count: usize) -> u64 {
    (value_count / 256 * 210) as u64
}

fn q5_0_storage_bytes(value_count: usize) -> u64 {
    (value_count / 32 * 22) as u64
}

fn push_q5_0_values(bytes: &mut Vec<u8>, values: &[f32]) {
    for block in values.chunks_exact(32) {
        bytes.extend_from_slice(&f32_to_f16_bits(1.0).to_le_bytes());
        let mut qh = 0u32;
        let mut qs = [0u8; 16];

        for index in 0..16 {
            let low = q5_0_quant(block[index]);
            let high = q5_0_quant(block[index + 16]);
            if low & 0x10 != 0 {
                qh |= 1 << index;
            }
            if high & 0x10 != 0 {
                qh |= 1 << (index + 16);
            }
            qs[index] = (low & 0x0f) | ((high & 0x0f) << 4);
        }

        bytes.extend_from_slice(&qh.to_le_bytes());
        bytes.extend_from_slice(&qs);
    }
}

fn q5_0_quant(value: f32) -> u8 {
    let quantized = value.round() as i32 + 16;
    quantized.clamp(0, 31) as u8
}

fn push_q4_k_values(bytes: &mut Vec<u8>, values: &[f32]) {
    for block in values.chunks_exact(256) {
        bytes.extend_from_slice(&f32_to_f16_bits(1.0).to_le_bytes());
        bytes.extend_from_slice(&f32_to_f16_bits(0.0).to_le_bytes());
        bytes.extend_from_slice(&q4_k_unit_scales());
        for chunk in block.chunks_exact(64) {
            for index in 0..32 {
                let low = chunk[index].round() as u8 & 0x0f;
                let high = (chunk[index + 32].round() as u8 & 0x0f) << 4;
                bytes.push(low | high);
            }
        }
    }
}

fn q4_k_unit_scales() -> [u8; 12] {
    [1, 1, 1, 1, 0, 0, 0, 0, 1, 1, 1, 1]
}

fn push_q6_k_values(bytes: &mut Vec<u8>, values: &[f32]) {
    for block in values.chunks_exact(256) {
        let mut low_bits = [0u8; 128];
        let mut high_bits = [0u8; 64];

        for half in 0..2 {
            let value_base = half * 128;
            let low_base = half * 64;
            let high_base = half * 32;

            for index in 0..32 {
                let q1 = q6_k_quant(block[value_base + index]);
                let q2 = q6_k_quant(block[value_base + index + 32]);
                let q3 = q6_k_quant(block[value_base + index + 64]);
                let q4 = q6_k_quant(block[value_base + index + 96]);

                low_bits[low_base + index] = (q1 & 0x0f) | ((q3 & 0x0f) << 4);
                low_bits[low_base + index + 32] = (q2 & 0x0f) | ((q4 & 0x0f) << 4);
                high_bits[high_base + index] = ((q1 >> 4) & 3)
                    | (((q2 >> 4) & 3) << 2)
                    | (((q3 >> 4) & 3) << 4)
                    | (((q4 >> 4) & 3) << 6);
            }
        }

        bytes.extend_from_slice(&low_bits);
        bytes.extend_from_slice(&high_bits);
        bytes.extend_from_slice(&[1u8; 16]);
        bytes.extend_from_slice(&f32_to_f16_bits(1.0).to_le_bytes());
    }
}

fn q6_k_quant(value: f32) -> u8 {
    let quantized = value.round() as i32 + 32;
    quantized.clamp(0, 63) as u8
}
