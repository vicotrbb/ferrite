use super::Matrix;

#[derive(Clone, Debug, PartialEq)]
pub enum ScalarLlamaOutputWeights {
    Untied(Matrix),
    TiedTokenEmbedding,
}

impl ScalarLlamaOutputWeights {
    pub fn untied(matrix: Matrix) -> Self {
        Self::Untied(matrix)
    }

    pub fn tied_token_embedding() -> Self {
        Self::TiedTokenEmbedding
    }

    pub(super) fn logits_matrix<'a>(&'a self, token_embedding: &'a Matrix) -> &'a Matrix {
        match self {
            Self::Untied(matrix) => matrix,
            Self::TiedTokenEmbedding => token_embedding,
        }
    }

    pub(super) fn untied_matrix(&self) -> Option<&Matrix> {
        match self {
            Self::Untied(matrix) => Some(matrix),
            Self::TiedTokenEmbedding => None,
        }
    }
}
