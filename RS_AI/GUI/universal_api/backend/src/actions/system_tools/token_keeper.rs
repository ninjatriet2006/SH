//! Token Keeper background daemon trigger action.

use crate::core::token_keeper::{run_keeper_cycle_once, TokenKeeperReport};
use crate::ipc::{respond, Empty, IpcResult, Req};

#[tauri::command(rename_all = "snake_case")]
pub fn trigger_token_keeper(request: Req<Empty>) -> IpcResult<TokenKeeperReport> {
    let (request_id, _) = request.validate()?;
    let report = run_keeper_cycle_once();
    Ok(respond(request_id, report))
}
