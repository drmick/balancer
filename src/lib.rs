mod round_robin;

pub use round_robin::StableRoundRobinBalancer;

use std::hash::Hash;
use std::sync::Arc;

pub trait Server: Send + Sync + 'static {
    type Id: Hash + Eq + Clone + Send + Sync;
    fn id(&self) -> &Self::Id;
}

#[trait_variant::make(Send)]
pub trait ServerBalancer<S, C = ()>: Send + Sync
where
    S: Server,
    C: Sync,
{
    async fn add_server(&self, server: S) -> bool;
    async fn remove_server(&self, id: &S::Id) -> Option<Arc<S>>;
    async fn get_next_server(&self, context: &C) -> Option<Arc<S>>;
}
