use std::collections::HashSet;
use std::sync::OnceLock;

// Known protocol infrastructure addresses sourced from reth_chain_query's
// canonical ROUTERS and POOL_FACTORIES maps. Built once, checked per tx.
static KNOWN_INFRASTRUCTURE: OnceLock<HashSet<String>> = OnceLock::new();

pub(crate) fn known_infrastructure() -> &'static HashSet<String> {
    KNOWN_INFRASTRUCTURE.get_or_init(|| {
        use reth_chain_query::common_addresses::{POOL_FACTORIES, ROUTERS};
        let mut set: HashSet<String> = ROUTERS.values().map(|addr| format!("{addr:#x}")).collect();
        if let Some(pool_manager) = POOL_FACTORIES.get("univ4_pool_manager") {
            set.insert(format!("{pool_manager:#x}"));
        }
        set
    })
}
