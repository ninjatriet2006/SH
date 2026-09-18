use std::sync::Arc;
use crate::proxy::AppState;
use crate::proxy::manager::ListenerManager;

pub struct ServerContext {
    pub app_state: Arc<AppState>,
    pub listener_mgr: Arc<ListenerManager>,
}
