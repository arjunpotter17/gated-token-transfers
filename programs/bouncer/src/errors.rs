use anchor_lang::prelude::*;

#[error_code]
pub enum BouncerError {
    #[msg("Invalid policy value")]
    InvalidPolicy,
    #[msg("Invalid storage kind value")]
    InvalidStorageKind,
    #[msg("Invalid status value")]
    InvalidStatus,
    #[msg("Unauthorized")]
    Unauthorized,
    #[msg("List is frozen")]
    Frozen,
    #[msg("Entry PDA mismatch")]
    EntryPdaMismatch,
    #[msg("Entry account has invalid owner")]
    EntryAccountInvalidOwner,
    #[msg("Entry data mismatch")]
    EntryDataMismatch,
    #[msg("Not allowed")]
    NotAllowed,
    #[msg("Proof not supported for this list kind")]
    ProofNotSupported,
    #[msg("Merkle config is invalid or uninitialized")]
    InvalidMerkleConfig,
    #[msg("Invalid merkle proof length")]
    InvalidProofLength,
    #[msg("Invalid merkle proof")]
    InvalidMerkleProof,
}
