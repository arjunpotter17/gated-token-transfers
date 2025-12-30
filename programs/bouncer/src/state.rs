use anchor_lang::prelude::*;

pub const LIST_VERSION: u8 = 1;
pub const ENTRY_VERSION: u8 = 1;

pub const POLICY_ALLOWLIST: u8 = 0;
pub const POLICY_BLOCKLIST: u8 = 1;

pub const STORAGE_DIRECT_PDA: u8 = 0;
pub const STORAGE_MERKLE_ROOT: u8 = 1;

pub const MERKLE_MAX_DEPTH: u8 = 64;

pub const ENTRY_STATUS_UNSET: u8 = 0;
pub const ENTRY_STATUS_ALLOW: u8 = 1;
pub const ENTRY_STATUS_BLOCK: u8 = 2;

pub const FLAG_FROZEN: u16 = 1 << 0;

#[account]
pub struct List {
    pub version: u8, // = 1
    pub bump: u8,

    pub authority: Pubkey,
    pub creator: Pubkey,
    pub list_id: u64,

    pub policy: u8,       // 0=Allowlist, 1=Blocklist
    pub storage_kind: u8, // 0=DirectPda, 1=SparseMerkleRoot, 2=ReservedCompressedTree
    pub flags: u16,

    pub entry_count: u32,
    pub reserved0: u32,

    pub storage_config: [u8; 96],
}

impl List {
    pub const LEN: usize = 256;

    pub fn is_frozen(&self) -> bool {
        (self.flags & FLAG_FROZEN) != 0
    }

    pub fn merkle_depth(&self) -> u8 {
        self.storage_config[0]
    }

    pub fn merkle_root(&self) -> [u8; 32] {
        let mut out = [0u8; 32];
        out.copy_from_slice(&self.storage_config[1..33]);
        out
    }

    pub fn set_merkle_config(&mut self, depth: u8, root: [u8; 32]) {
        self.storage_config[0] = depth;
        self.storage_config[1..33].copy_from_slice(&root);
    }
}

#[account]
pub struct Entry {
    pub version: u8, // = 1
    pub bump: u8,
    pub status: u8, // 0=Unset (should not exist), 1=Allow, 2=Block
    pub reserved: u8,

    pub list: Pubkey,
    pub subject: Pubkey,
}

impl Entry {
    pub const LEN: usize = 96;
}

#[event]
pub struct ListCreated {
    pub list: Pubkey,
    pub creator: Pubkey,
    pub authority: Pubkey,
    pub list_id: u64,
    pub policy: u8,
    pub storage_kind: u8,
}

#[event]
pub struct AuthorityChanged {
    pub list: Pubkey,
    pub old_authority: Pubkey,
    pub new_authority: Pubkey,
}

#[event]
pub struct PolicyChanged {
    pub list: Pubkey,
    pub old_policy: u8,
    pub new_policy: u8,
}

#[event]
pub struct ListFrozen {
    pub list: Pubkey,
}

#[event]
pub struct EntryUpserted {
    pub list: Pubkey,
    pub subject: Pubkey,
    pub status: u8,
}

#[event]
pub struct EntryRemoved {
    pub list: Pubkey,
    pub subject: Pubkey,
}

#[event]
pub struct MerkleConfigSet {
    pub list: Pubkey,
    pub authority: Pubkey,
    pub depth: u8,
    pub root: [u8; 32],
}
