use crate::gguf_writer::{
    F32TensorFixture, GGML_TYPE_F32, GGML_TYPE_Q4_K, GGML_TYPE_Q5_0, GGML_TYPE_Q6_K,
    TypedTensorFixture,
};

pub(crate) fn q8_scalar_tensors() -> Vec<F32TensorFixture> {
    let hidden = 32usize;
    let intermediate = 32usize;
    vec![
        F32TensorFixture {
            name: "token_embd.weight",
            dimensions: matrix_dims(hidden as u64, 2),
            values: two_row_values(hidden, 1.0, 0.0),
            offset: 0,
        },
        F32TensorFixture {
            name: "output_norm.weight",
            dimensions: vec![hidden as u64],
            values: vec![1.0; hidden],
            offset: 0,
        },
        F32TensorFixture {
            name: "output.weight",
            dimensions: matrix_dims(hidden as u64, 2),
            values: two_row_values(hidden, 0.0, 1.0),
            offset: 0,
        },
        F32TensorFixture {
            name: "blk.0.attn_norm.weight",
            dimensions: vec![hidden as u64],
            values: vec![1.0; hidden],
            offset: 0,
        },
        F32TensorFixture {
            name: "blk.0.attn_q.weight",
            dimensions: matrix_dims(hidden as u64, hidden as u64),
            values: vec![0.0; hidden * hidden],
            offset: 0,
        },
        F32TensorFixture {
            name: "blk.0.attn_k.weight",
            dimensions: matrix_dims(hidden as u64, hidden as u64),
            values: vec![0.0; hidden * hidden],
            offset: 0,
        },
        F32TensorFixture {
            name: "blk.0.attn_v.weight",
            dimensions: matrix_dims(hidden as u64, hidden as u64),
            values: vec![0.0; hidden * hidden],
            offset: 0,
        },
        F32TensorFixture {
            name: "blk.0.attn_output.weight",
            dimensions: matrix_dims(hidden as u64, hidden as u64),
            values: vec![0.0; hidden * hidden],
            offset: 0,
        },
        F32TensorFixture {
            name: "blk.0.ffn_norm.weight",
            dimensions: vec![hidden as u64],
            values: vec![1.0; hidden],
            offset: 0,
        },
        F32TensorFixture {
            name: "blk.0.ffn_gate.weight",
            dimensions: matrix_dims(hidden as u64, intermediate as u64),
            values: vec![0.0; hidden * intermediate],
            offset: 0,
        },
        F32TensorFixture {
            name: "blk.0.ffn_up.weight",
            dimensions: matrix_dims(hidden as u64, intermediate as u64),
            values: vec![0.0; hidden * intermediate],
            offset: 0,
        },
        F32TensorFixture {
            name: "blk.0.ffn_down.weight",
            dimensions: matrix_dims(intermediate as u64, hidden as u64),
            values: vec![0.0; hidden * intermediate],
            offset: 0,
        },
    ]
}

pub(crate) fn q4_k_scalar_tensors() -> Vec<TypedTensorFixture> {
    let hidden = 64usize;
    let intermediate = 64usize;
    vec![
        q4_k_tensor(
            "token_embd.weight",
            matrix_dims(hidden as u64, 4),
            four_row_values(hidden, &[1.0, 1.0, 0.0, 0.0]),
        ),
        f32_tensor("output_norm.weight", vec![hidden as u64], vec![1.0; hidden]),
        q4_k_tensor(
            "output.weight",
            matrix_dims(hidden as u64, 4),
            four_row_values(hidden, &[0.0, 1.0, 0.0, 0.0]),
        ),
        f32_tensor(
            "blk.0.attn_norm.weight",
            vec![hidden as u64],
            vec![1.0; hidden],
        ),
        q4_k_tensor(
            "blk.0.attn_q.weight",
            matrix_dims(hidden as u64, hidden as u64),
            vec![0.0; hidden * hidden],
        ),
        q4_k_tensor(
            "blk.0.attn_k.weight",
            matrix_dims(hidden as u64, hidden as u64),
            vec![0.0; hidden * hidden],
        ),
        q4_k_tensor(
            "blk.0.attn_v.weight",
            matrix_dims(hidden as u64, hidden as u64),
            vec![0.0; hidden * hidden],
        ),
        q4_k_tensor(
            "blk.0.attn_output.weight",
            matrix_dims(hidden as u64, hidden as u64),
            vec![0.0; hidden * hidden],
        ),
        f32_tensor(
            "blk.0.ffn_norm.weight",
            vec![hidden as u64],
            vec![1.0; hidden],
        ),
        q4_k_tensor(
            "blk.0.ffn_gate.weight",
            matrix_dims(hidden as u64, intermediate as u64),
            vec![0.0; hidden * intermediate],
        ),
        q4_k_tensor(
            "blk.0.ffn_up.weight",
            matrix_dims(hidden as u64, intermediate as u64),
            vec![0.0; hidden * intermediate],
        ),
        q4_k_tensor(
            "blk.0.ffn_down.weight",
            matrix_dims(intermediate as u64, hidden as u64),
            vec![0.0; hidden * intermediate],
        ),
    ]
}

pub(crate) fn q6_k_scalar_tensors() -> Vec<TypedTensorFixture> {
    q4_k_scalar_tensors()
        .into_iter()
        .map(|tensor| TypedTensorFixture {
            name: tensor.name,
            dimensions: tensor.dimensions,
            values: tensor.values,
            tensor_type: if tensor.tensor_type == GGML_TYPE_Q4_K {
                GGML_TYPE_Q6_K
            } else {
                tensor.tensor_type
            },
            offset: 0,
        })
        .collect()
}

pub(crate) fn q5_0_scalar_tensors() -> Vec<TypedTensorFixture> {
    q8_scalar_tensors()
        .into_iter()
        .map(|tensor| TypedTensorFixture {
            name: tensor.name,
            dimensions: tensor.dimensions,
            values: tensor.values,
            tensor_type: GGML_TYPE_Q5_0,
            offset: 0,
        })
        .collect()
}

pub(crate) fn matrix_dims(cols: u64, rows: u64) -> Vec<u64> {
    vec![cols, rows]
}

fn q4_k_tensor(name: &'static str, dimensions: Vec<u64>, values: Vec<f32>) -> TypedTensorFixture {
    TypedTensorFixture {
        name,
        dimensions,
        values,
        tensor_type: GGML_TYPE_Q4_K,
        offset: 0,
    }
}

fn f32_tensor(name: &'static str, dimensions: Vec<u64>, values: Vec<f32>) -> TypedTensorFixture {
    TypedTensorFixture {
        name,
        dimensions,
        values,
        tensor_type: GGML_TYPE_F32,
        offset: 0,
    }
}

fn two_row_values(cols: usize, first: f32, second: f32) -> Vec<f32> {
    let mut values = vec![first; cols];
    values.extend(vec![second; cols]);
    values
}

fn four_row_values(cols: usize, row_values: &[f32; 4]) -> Vec<f32> {
    let mut values = Vec::with_capacity(cols * row_values.len());
    for row_value in row_values {
        values.extend(vec![*row_value; cols]);
    }
    values
}
