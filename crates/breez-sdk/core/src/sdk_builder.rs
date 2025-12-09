#![cfg_attr(
    all(target_family = "wasm", target_os = "unknown"),
    allow(clippy::arc_with_non_send_sync)
)]
use std::sync::Arc;

use bitcoin::bip32::Xpriv;
use breez_sdk_common::{
    breez_server::{BreezServer, PRODUCTION_BREEZSERVER_URL},
    rest::ReqwestRestClient as CommonRequestRestClient,
};
use spark_wallet::{DefaultSigner, KeySet, Signer};
use tokio::sync::watch;
use tracing::{debug, info};

use crate::{
    Credentials, EventEmitter, FiatService, FiatServiceWrapper, KeySetType, Network, RestClient,
    RestClientWrapper, Seed,
    chain::{
        BitcoinChainService,
        rest_client::{BasicAuth, ChainApiType, RestClientChainService},
    },
    error::SdkError,
    lnurl::{LnurlServerClient, ReqwestLnurlServerClient},
    models::Config,
    nostr::NostrClient,
    payment_observer::{PaymentObserver, SparkTransferObserver},
    persist::Storage,
    realtime_sync::{RealTimeSyncParams, init_and_start_real_time_sync},
    sdk::{BreezSdk, BreezSdkParams},
    sync_storage::SyncStorage,
};

/// Builder for creating `BreezSdk` instances with customizable components.
#[derive(Clone)]
pub struct SdkBuilder {
    config: Config,
    seed: Option<Seed>,
    signer: Option<Arc<dyn Signer>>,
    identity_master_key: Option<Xpriv>,
    storage_dir: Option<String>,
    storage: Option<Arc<dyn Storage>>,
    chain_service: Option<Arc<dyn BitcoinChainService>>,
    fiat_service: Option<Arc<dyn FiatService>>,
    lnurl_client: Option<Arc<dyn RestClient>>,
    lnurl_server_client: Option<Arc<dyn LnurlServerClient>>,
    payment_observer: Option<Arc<dyn PaymentObserver>>,
    key_set_type: KeySetType,
    use_address_index: bool,
    account_number: Option<u32>,
    sync_storage: Option<Arc<dyn SyncStorage>>,
}

impl SdkBuilder {
    /// Creates a new `SdkBuilder` with the provided configuration and seed.
    /// Arguments:
    /// - `config`: The configuration to be used.
    /// - `seed`: The seed for wallet generation.
    pub fn new(config: Config, seed: Seed) -> Self {
        SdkBuilder {
            config,
            seed: Some(seed),
            signer: None,
            identity_master_key: None,
            storage_dir: None,
            storage: None,
            chain_service: None,
            fiat_service: None,
            lnurl_client: None,
            lnurl_server_client: None,
            payment_observer: None,
            key_set_type: KeySetType::Default,
            use_address_index: false,
            account_number: None,
            sync_storage: None,
        }
    }

    /// Creates a new `SdkBuilder` with an external signer.
    ///
    /// Use this constructor when you want to manage keys externally (e.g., in a secure enclave
    /// or hardware wallet) instead of deriving them from a seed.
    ///
    /// **Note**: When using an external signer:
    /// - Real-time sync is disabled (requires master key derivation)
    /// - Nostr client is disabled unless `identity_master_key` is provided via [`with_identity_master_key`]
    ///
    /// Arguments:
    /// - `config`: The configuration to be used.
    /// - `signer`: The external signer implementation.
    pub fn new_with_signer(config: Config, signer: Arc<dyn Signer>) -> Self {
        SdkBuilder {
            config,
            seed: None,
            signer: Some(signer),
            identity_master_key: None,
            storage_dir: None,
            storage: None,
            chain_service: None,
            fiat_service: None,
            lnurl_client: None,
            lnurl_server_client: None,
            payment_observer: None,
            key_set_type: KeySetType::Default,
            use_address_index: false,
            account_number: None,
            sync_storage: None,
        }
    }

    /// Sets the identity master key for real-time sync and Nostr client.
    ///
    /// This is only needed when using an external signer and you want to enable:
    /// - Real-time sync (multi-device payment metadata sync)
    /// - Nostr client functionality
    ///
    /// When using `new()` with a seed, this is derived automatically.
    #[must_use]
    pub fn with_identity_master_key(mut self, master_key: Xpriv) -> Self {
        self.identity_master_key = Some(master_key);
        self
    }

    #[must_use]
    /// Sets the root storage directory to initialize the default storage with.
    /// This initializes both storage and real-time sync storage with the
    /// default implementations.
    /// Arguments:
    /// - `storage_dir`: The data directory for storage.
    pub fn with_default_storage(mut self, storage_dir: String) -> Self {
        self.storage_dir = Some(storage_dir);
        self
    }

    #[must_use]
    /// Sets the storage implementation to be used by the SDK.
    /// Arguments:
    /// - `storage`: The storage implementation to be used.
    pub fn with_storage(mut self, storage: Arc<dyn Storage>) -> Self {
        self.storage = Some(storage);
        self
    }

    #[must_use]
    /// Sets the real-time sync storage implementation to be used by the SDK.
    /// Arguments:
    /// - `storage`: The sync storage implementation to be used.
    pub fn with_real_time_sync_storage(mut self, storage: Arc<dyn SyncStorage>) -> Self {
        self.sync_storage = Some(storage);
        self
    }

    /// Sets the key set type to be used by the SDK.
    /// Arguments:
    /// - `key_set_type`: The key set type which determines the derivation path.
    /// - `use_address_index`: Controls the structure of the BIP derivation path.
    #[must_use]
    pub fn with_key_set(
        mut self,
        key_set_type: KeySetType,
        use_address_index: bool,
        account_number: Option<u32>,
    ) -> Self {
        self.key_set_type = key_set_type;
        self.use_address_index = use_address_index;
        self.account_number = account_number;
        self
    }

    /// Sets the chain service to be used by the SDK.
    /// Arguments:
    /// - `chain_service`: The chain service to be used.
    #[must_use]
    pub fn with_chain_service(mut self, chain_service: Arc<dyn BitcoinChainService>) -> Self {
        self.chain_service = Some(chain_service);
        self
    }

    /// Sets the REST chain service to be used by the SDK.
    /// Arguments:
    /// - `url`: The base URL of the REST API.
    /// - `api_type`: The API type to be used.
    /// - `credentials`: Optional credentials for basic authentication.
    #[must_use]
    pub fn with_rest_chain_service(
        mut self,
        url: String,
        api_type: ChainApiType,
        credentials: Option<Credentials>,
    ) -> Self {
        self.chain_service = Some(Arc::new(RestClientChainService::new(
            url,
            self.config.network,
            5,
            Box::new(CommonRequestRestClient::new().unwrap()),
            credentials.map(|c| BasicAuth::new(c.username, c.password)),
            api_type,
        )));
        self
    }

    /// Sets the fiat service to be used by the SDK.
    /// Arguments:
    /// - `fiat_service`: The fiat service to be used.
    #[must_use]
    pub fn with_fiat_service(mut self, fiat_service: Arc<dyn FiatService>) -> Self {
        self.fiat_service = Some(fiat_service);
        self
    }

    #[must_use]
    pub fn with_lnurl_client(mut self, lnurl_client: Arc<dyn RestClient>) -> Self {
        self.lnurl_client = Some(lnurl_client);
        self
    }

    #[must_use]
    #[allow(unused)]
    pub fn with_lnurl_server_client(
        mut self,
        lnurl_serverclient: Arc<dyn LnurlServerClient>,
    ) -> Self {
        self.lnurl_server_client = Some(lnurl_serverclient);
        self
    }

    /// Sets the payment observer to be used by the SDK.
    /// This observer will receive callbacks before outgoing payments for Lightning, Spark and onchain Bitcoin.
    /// Arguments:
    /// - `payment_observer`: The payment observer to be used.
    #[must_use]
    #[allow(unused)]
    pub fn with_payment_observer(mut self, payment_observer: Arc<dyn PaymentObserver>) -> Self {
        self.payment_observer = Some(payment_observer);
        self
    }

    /// Builds the `BreezSdk` instance with the configured components.
    #[allow(clippy::too_many_lines)]
    pub async fn build(self) -> Result<BreezSdk, SdkError> {
        // Create signer and key_set based on configuration
        let (signer, key_set): (Arc<dyn Signer>, Option<KeySet>) = match (&self.signer, &self.seed) {
            // External signer provided
            (Some(signer), _) => (Arc::clone(signer), None),
            // Seed provided - derive signer from seed
            (None, Some(seed)) => {
                let seed_bytes = seed.to_bytes()?;
                let key_set = KeySet::new(
                    &seed_bytes,
                    self.config.network.into(),
                    self.key_set_type.into(),
                    self.use_address_index,
                    self.account_number,
                )
                .map_err(|e| SdkError::Generic(e.to_string()))?;
                let signer: Arc<dyn Signer> = Arc::new(DefaultSigner::from_key_set(key_set.clone()));
                (signer, Some(key_set))
            }
            // Neither provided - error
            (None, None) => {
                return Err(SdkError::Generic(
                    "Either seed or signer must be provided to build the SDK".to_string(),
                ));
            }
        };

        let chain_service = if let Some(service) = self.chain_service {
            service
        } else {
            let inner_client =
                CommonRequestRestClient::new().map_err(|e| SdkError::Generic(e.to_string()))?;
            match self.config.network {
                Network::Mainnet => Arc::new(RestClientChainService::new(
                    "https://blockstream.info/api".to_string(),
                    self.config.network,
                    5,
                    Box::new(inner_client),
                    None,
                    ChainApiType::Esplora,
                )),
                Network::Regtest => Arc::new(RestClientChainService::new(
                    "https://regtest-mempool.us-west-2.sparkinfra.net/api".to_string(),
                    self.config.network,
                    5,
                    Box::new(inner_client),
                    match (
                        std::env::var("CHAIN_SERVICE_USERNAME"),
                        std::env::var("CHAIN_SERVICE_PASSWORD"),
                    ) {
                        (Ok(username), Ok(password)) => Some(BasicAuth::new(username, password)),
                        _ => Some(BasicAuth::new(
                            "spark-sdk".to_string(),
                            "mCMk1JqlBNtetUNy".to_string(),
                        )),
                    },
                    ChainApiType::MempoolSpace,
                )),
            }
        };

        let (storage, sync_storage) = match (self.storage, self.storage_dir) {
            // Use provided storages directly
            (Some(storage), _) => (storage, self.sync_storage),
            // Initialize default storages based on provided directory
            #[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
            (None, Some(storage_dir)) => {
                let identity_pub_key = signer
                    .get_identity_public_key()
                    .map_err(|e| SdkError::Generic(e.to_string()))?;
                let storage =
                    default_storage(&storage_dir, self.config.network, &identity_pub_key)?;
                let sync_storage = match (self.sync_storage, &self.config.real_time_sync_server_url)
                {
                    // Use provided sync storage directly
                    (Some(sync_storage), _) => Some(sync_storage),
                    // Initialize default sync storage based on provided directory
                    // if real-time sync is enabled
                    (None, Some(_)) => Some(default_sync_storage(
                        &storage_dir,
                        self.config.network,
                        &identity_pub_key,
                    )?),
                    _ => None,
                };
                (storage, sync_storage)
            }
            _ => {
                return Err(SdkError::Generic(
                    "Either storage or storage_dir must be set before building the SDK".to_string(),
                ));
            }
        };

        let fiat_service: Arc<dyn breez_sdk_common::fiat::FiatService> = match self.fiat_service {
            Some(service) => Arc::new(FiatServiceWrapper::new(service)),
            None => Arc::new(
                BreezServer::new(PRODUCTION_BREEZSERVER_URL, None)
                    .map_err(|e| SdkError::Generic(e.to_string()))?,
            ),
        };

        let lnurl_client: Arc<dyn breez_sdk_common::rest::RestClient> = match self.lnurl_client {
            Some(client) => Arc::new(RestClientWrapper::new(client)),
            None => Arc::new(
                CommonRequestRestClient::new().map_err(|e| SdkError::Generic(e.to_string()))?,
            ),
        };
        let user_agent = format!(
            "{}/{}",
            crate::built_info::PKG_NAME,
            crate::built_info::GIT_VERSION.unwrap_or(crate::built_info::PKG_VERSION),
        );
        info!("Building SparkWallet with user agent: {}", user_agent);
        let mut spark_wallet_config =
            spark_wallet::SparkWalletConfig::default_config(self.config.network.into());
        spark_wallet_config.operator_pool = spark_wallet_config
            .operator_pool
            .with_user_agent(Some(user_agent.clone()));
        spark_wallet_config.service_provider_config.user_agent = Some(user_agent);

        let mut wallet_builder =
            spark_wallet::WalletBuilder::new(spark_wallet_config, Arc::clone(&signer));
        if let Some(observer) = self.payment_observer {
            let observer: Arc<dyn spark_wallet::TransferObserver> =
                Arc::new(SparkTransferObserver::new(observer));
            wallet_builder = wallet_builder.with_transfer_observer(observer);
        }
        let spark_wallet = Arc::new(wallet_builder.build().await?);

        let lnurl_server_client: Option<Arc<dyn LnurlServerClient>> = match self.lnurl_server_client
        {
            Some(client) => Some(client),
            None => match &self.config.lnurl_domain {
                Some(domain) => {
                    // Get the SparkWallet instance for signing
                    Some(Arc::new(ReqwestLnurlServerClient::new(
                        domain.clone(),
                        self.config.api_key.clone(),
                        Arc::clone(&spark_wallet),
                    )?))
                }
                None => None,
            },
        };
        let shutdown_sender = watch::channel::<()>(()).0;

        // Determine the identity master key for real-time sync and nostr
        // Priority: 1) explicitly provided, 2) derived from seed via key_set
        let identity_master_key: Option<Xpriv> = match (&self.identity_master_key, &key_set, &self.seed) {
            // Explicitly provided master key
            (Some(master_key), _, _) => Some(master_key.clone()),
            // Derived from seed - use legacy derivation for backwards compatibility
            (None, Some(_), Some(seed)) if self.key_set_type == KeySetType::Default && self.account_number.is_none() => {
                let seed_bytes = seed.to_bytes()?;
                let bitcoin_network: bitcoin::Network = self.config.network.into();
                Some(Xpriv::new_master(bitcoin_network, &seed_bytes)
                    .map_err(|e| SdkError::Generic(e.to_string()))?)
            }
            // Derived from key_set
            (None, Some(ks), _) => Some(ks.identity_master_key.clone()),
            // External signer without master key - features requiring master key are disabled
            (None, None, _) => {
                info!("External signer without identity_master_key: real-time sync and nostr will be disabled");
                None
            }
        };

        // Determine if real-time sync can be enabled
        let has_real_time_sync = self.config.real_time_sync_server_url.is_some() && identity_master_key.is_some();
        let event_emitter = Arc::new(EventEmitter::new(has_real_time_sync));

        let storage = match (&self.config.real_time_sync_server_url, &identity_master_key, sync_storage) {
            // Real-time sync enabled and we have master key
            (Some(server_url), Some(master_key), Some(sync_storage)) => {
                init_and_start_real_time_sync(RealTimeSyncParams {
                    server_url: server_url.clone(),
                    api_key: self.config.api_key.clone(),
                    network: self.config.network,
                    master_key: master_key.clone(),
                    storage: Arc::clone(&storage),
                    sync_storage,
                    shutdown_receiver: shutdown_sender.subscribe(),
                    event_emitter: Arc::clone(&event_emitter),
                })
                .await?
            }
            // Real-time sync URL configured but no master key (external signer)
            (Some(_), None, _) => {
                info!("Real-time sync disabled: external signer without identity_master_key");
                storage
            }
            // Real-time sync URL configured but no sync storage
            (Some(_), Some(_), None) => {
                return Err(SdkError::Generic(
                    "Real-time sync is enabled, but no sync storage is supplied".to_string(),
                ));
            }
            // No real-time sync configured
            _ => storage,
        };

        // Create nostr client if we have the master key
        let nostr_client: Option<Arc<NostrClient>> = match identity_master_key {
            Some(ref master_key) => {
                let account = self.account_number.unwrap_or(match self.config.network {
                    Network::Mainnet => 0,
                    Network::Regtest => 1,
                });
                Some(Arc::new(NostrClient::new(master_key, account)?))
            }
            None => None,
        };

        // Create the SDK instance
        let sdk = BreezSdk::init_and_start(BreezSdkParams {
            config: self.config,
            storage,
            chain_service,
            fiat_service,
            lnurl_client,
            lnurl_server_client,
            shutdown_sender,
            spark_wallet,
            event_emitter,
            nostr_client,
        })?;
        debug!("Initialized and started breez sdk.");

        Ok(sdk)
    }
}

#[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
fn default_storage(
    data_dir: &str,
    network: Network,
    identity_pub_key: &spark_wallet::PublicKey,
) -> Result<Arc<dyn Storage>, SdkError> {
    let db_path = crate::default_storage_path(data_dir, &network, identity_pub_key)?;
    let storage = Arc::new(crate::SqliteStorage::new(&db_path)?);
    Ok(storage)
}

#[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
fn default_sync_storage(
    data_dir: &str,
    network: Network,
    identity_pub_key: &spark_wallet::PublicKey,
) -> Result<Arc<dyn SyncStorage>, SdkError> {
    let db_path = crate::default_storage_path(data_dir, &network, identity_pub_key)?;
    let storage = Arc::new(crate::SqliteStorage::new(&db_path)?);
    Ok(storage)
}
