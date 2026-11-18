const DISABLED: &str = "token support is not compiled in, enable the `tokens` feature";

#[derive(Debug, Clone)]
pub struct SelectedTokenOutput {
    pub previous_transaction_hash: Vec<u8>,
    pub previous_transaction_vout: u32,
    pub owner_public_key: Vec<u8>,
    pub token_identifier: Vec<u8>,
    pub token_amount: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct ReceiverTokenOutput {
    pub receiver_spark_address: String,
    pub token_identifier: Option<Vec<u8>>,
    pub token_amount: Option<Vec<u8>>,
}

#[derive(Debug, Clone)]
pub struct TransferBuildRequest {
    pub identity_public_key: Vec<u8>,
    pub selected_outputs: Vec<SelectedTokenOutput>,
    pub receiver_outputs: Vec<ReceiverTokenOutput>,
    pub operator_identity_public_keys: Vec<Vec<u8>>,
    pub network: u32,
    pub validity_duration_seconds: u64,
    pub client_created_timestamp_unix_micros: i64,
    pub withdraw_bond_sats: u64,
    pub withdraw_relative_block_locktime: u64,
    pub execute_before_unix_micros: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct CreateBuildRequest {
    pub issuer_public_key: Vec<u8>,
    pub token_name: String,
    pub token_ticker: String,
    pub decimals: u32,
    pub max_supply: Vec<u8>,
    pub is_freezable: bool,
    pub extra_metadata: Option<Vec<u8>>,
    pub operator_identity_public_keys: Vec<Vec<u8>>,
    pub network: u32,
    pub validity_duration_seconds: u64,
    pub client_created_timestamp_unix_micros: i64,
    pub execute_before_unix_micros: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct MintBuildRequest {
    pub issuer_public_key: Vec<u8>,
    pub token_identifier: Vec<u8>,
    pub token_amount: Vec<u8>,
    pub receiver_public_key: Option<Vec<u8>>,
    pub operator_identity_public_keys: Vec<Vec<u8>>,
    pub network: u32,
    pub validity_duration_seconds: u64,
    pub client_created_timestamp_unix_micros: i64,
    pub withdraw_bond_sats: u64,
    pub withdraw_relative_block_locktime: u64,
    pub execute_before_unix_micros: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct PartialTransferBuildResult {
    pub partial_token_transaction_bytes: Vec<u8>,
    pub partial_token_transaction_hash: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct SignatureWithIndexInput {
    pub input_index: u32,
    pub public_key: Vec<u8>,
    pub signature: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct BroadcastBuildRequest {
    pub identity_public_key: Vec<u8>,
    pub partial_token_transaction_bytes: Vec<u8>,
    pub owner_signatures: Vec<SignatureWithIndexInput>,
}

#[derive(Debug, Clone)]
pub struct PrepareTokenInvoiceRequest {
    pub receiver_identity_public_key: Vec<u8>,
    pub network: u32,
    pub token_identifier: Option<Vec<u8>>,
    pub token_amount: Option<Vec<u8>>,
    pub memo: Option<String>,
    pub sender_spark_address: Option<String>,
    pub expiry_time_unix_millis: Option<u64>,
    pub invoice_id: Option<Vec<u8>>,
}

#[derive(Debug, Clone)]
pub struct PreparedTokenInvoice {
    pub spark_invoice_fields_bytes: Vec<u8>,
    pub spark_invoice_hash: Vec<u8>,
    pub unsigned_spark_address: String,
}

#[derive(Debug, Clone)]
pub struct FinalizeTokenInvoiceRequest {
    pub receiver_identity_public_key: Vec<u8>,
    pub network: u32,
    pub spark_invoice_fields_bytes: Vec<u8>,
    pub signature: Option<Vec<u8>>,
}

pub fn construct_partial_transfer_transaction(
    _request: TransferBuildRequest,
) -> Result<PartialTransferBuildResult, &'static str> {
    Err(DISABLED)
}

pub fn construct_partial_create_transaction(
    _request: CreateBuildRequest,
) -> Result<PartialTransferBuildResult, &'static str> {
    Err(DISABLED)
}

pub fn construct_partial_mint_transaction(
    _request: MintBuildRequest,
) -> Result<PartialTransferBuildResult, &'static str> {
    Err(DISABLED)
}

pub fn hash_final_token_transaction(
    _final_token_transaction_bytes: Vec<u8>,
) -> Result<Vec<u8>, &'static str> {
    Err(DISABLED)
}

pub fn build_broadcast_transaction_request(
    _request: BroadcastBuildRequest,
) -> Result<Vec<u8>, &'static str> {
    Err(DISABLED)
}

pub fn prepare_token_invoice(
    _request: PrepareTokenInvoiceRequest,
) -> Result<PreparedTokenInvoice, &'static str> {
    Err(DISABLED)
}

pub fn finalize_token_invoice(
    _request: FinalizeTokenInvoiceRequest,
) -> Result<String, &'static str> {
    Err(DISABLED)
}
