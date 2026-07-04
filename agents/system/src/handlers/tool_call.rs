use crate::{prelude::*, tools};
use json::from_value as parse;

/// API: Handles the agent tool call
#[log(skip_all, fields(tool = %name.0))]
pub async fn handle_tool_call(name: Paths<String>, data: Json<JsonValue>) -> Response {
    info!("Initialized the `{}` tool handling", name.0);

    Response::ok().stream(async move |tx| {
        if let Err(e) = tool_call(tx.clone(), name.0, data.0).await {
            error!("{e}");
            tx.send(Chunk::error(e.to_string())).await.ok();
        }
    })
}

async fn tool_call(tx: Sender<Bytes>, name: String, data: JsonValue) -> Result<()> {
    match name.as_ref() {
        // system monitor
        "get_system_info" => tools::handle_system_info(tx.clone()).await,
        "get_system_metrics" => tools::handle_system_metrics(tx.clone()).await,
        "get_devices_list" => tools::handle_devices_list(tx.clone()).await,

        // system theme
        "set_theme" => tools::handle_set_theme(tx.clone(), parse(data)?).await,
        // TODO: "get_theme" => tools::handle_get_theme(tx.clone(), parse(data)?).await,

        // power management
        "schedule_power" => tools::handle_schedule_power(tx.clone(), parse(data)?).await,
        "cancel_power" => tools::handle_cancel_power(tx.clone()).await,
        "power_status" => tools::handle_power_status(tx.clone()).await,

        // audio control
        "get_volume" => tools::handle_get_volume(tx.clone()).await,
        "set_volume" => tools::handle_set_volume(tx.clone(), parse(data)?).await,
        "increase_volume" => tools::handle_increase_volume(tx.clone(), parse(data)?).await,
        "decrease_volume" => tools::handle_decrease_volume(tx.clone(), parse(data)?).await,
        "is_muted" => tools::handle_is_muted(tx.clone()).await,
        "set_mute" => tools::handle_set_mute(tx.clone(), parse(data)?).await,

        // music indexer
        "search_music" => tools::handle_search_music(tx.clone(), parse(data)?).await,
        "play_music" => tools::handle_play_music(tx.clone(), parse(data)?).await,

        _ => Err(Error::UnknownTool(name).into()),
    }
}
