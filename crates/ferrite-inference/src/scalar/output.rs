use super::Matrix;

#[derive(Clone, Debug, PartialEq)]
/// Selects whether output projection weights are independent or tied.
pub enum ScalarLlamaOutputWeights {
    /// A dedicated vocabulary projection matrix.
    Untied(Matrix),
    /// Reuse the token embedding matrix for vocabulary logits.
    TiedTokenEmbedding,
}

impl ScalarLlamaOutputWeights {
    /// Wraps a dedicated vocabulary projection matrix.
    pub fn untied(matrix: Matrix) -> Self {
        Self::Untied(matrix)
    }

    /// Selects the token embedding matrix as the vocabulary projection.
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
