use std::collections::BTreeMap;
use std::sync::RwLock;

use once_cell::sync::OnceCell;

#[derive(Debug)]
pub struct AssetError {
    pub message: String,
}

pub trait AssetResolver: Send + Sync {
    fn resolve(&self, name: &str) -> Result<Option<String>, AssetError>;
}

type HostResolverFn = dyn Fn(&str) -> Option<String> + Send + Sync;

pub struct MapResolver {
    map: BTreeMap<String, String>,
}

impl MapResolver {
    pub fn new(map: BTreeMap<String, String>) -> Self {
        Self { map }
    }
}

impl AssetResolver for MapResolver {
    fn resolve(&self, name: &str) -> Result<Option<String>, AssetError> {
        Ok(self.map.get(name).cloned())
    }
}

pub struct CallbackResolver {
    callback: Box<HostResolverFn>,
}

impl CallbackResolver {
    pub fn new(callback: Box<HostResolverFn>) -> Self {
        Self { callback }
    }
}

impl AssetResolver for CallbackResolver {
    fn resolve(&self, name: &str) -> Result<Option<String>, AssetError> {
        Ok((self.callback)(name))
    }
}

static HOST_RESOLVER: OnceCell<RwLock<Option<Box<dyn AssetResolver>>>> = OnceCell::new();

fn host_cell() -> &'static RwLock<Option<Box<dyn AssetResolver>>> {
    HOST_RESOLVER.get_or_init(|| RwLock::new(None))
}

pub fn register_host_asset_resolver(resolver: Box<dyn AssetResolver>) -> Result<(), &'static str> {
    *host_cell().write().map_err(|_| "host resolver poisoned")? = Some(resolver);
    Ok(())
}

pub fn register_host_asset_map(map: BTreeMap<String, String>) -> Result<(), &'static str> {
    register_host_asset_resolver(Box::new(MapResolver::new(map)))
}

pub fn register_host_asset_callback(cb: Box<HostResolverFn>) -> Result<(), &'static str> {
    register_host_asset_resolver(Box::new(CallbackResolver::new(cb)))
}

pub fn resolve_with_host(name: &str) -> Result<Option<String>, AssetError> {
    if let Ok(guard) = host_cell().read()
        && let Some(resolver) = guard.as_ref()
    {
        return resolver.resolve(name);
    }
    Ok(None)
}
