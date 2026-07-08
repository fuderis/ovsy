use crate::{prelude::*, tools};
use json::from_value as parse;

/// API: Handles the agent tool call
#[log(skip_all, fields(tool = %name.0))]
pub async fn handle_tool_call(name: Paths<String>, data: Json<JsonValue>) -> Response {
    info!("Initialized the `{}` tool handling", name.0);

    Response::ok().stream(async move |tx| {
        if let Err(e) = tool_call(tx.clone(), name.0, data.0).await {
            error!("{e}");
            tx.send(Chunk::error(e.to_string())).ok();
        }
    })
}

async fn tool_call(tx: Sender<Bytes>, name: String, data: JsonValue) -> Result<()> {
    match name.as_ref() {
        //     SYSTEM MONITOR
        "get_system_info" => tools::monitor::handle_system_info(tx.clone()).await,
        "get_system_metrics" => tools::monitor::handle_system_metrics(tx.clone()).await,
        "get_devices_list" => tools::monitor::handle_devices_list(tx.clone()).await,

        //     SYSTEM THEME
        "set_theme" => tools::theme::handle_set_theme(tx.clone(), parse(data)?).await,
        // TODO: "get_theme" => tools::handle_get_theme(tx.clone(), parse(data)?).await,

        //     POWER MANAGEMENT
        "schedule_power" => tools::power::handle_schedule_power(tx.clone(), parse(data)?).await,
        "cancel_power" => tools::power::handle_cancel_power(tx.clone()).await,
        "get_power_status" => tools::power::handle_power_status(tx.clone()).await,

        //     AUDIO CONTROL
        "get_volume" => tools::audio::handle_get_volume(tx.clone()).await,
        "set_volume" => tools::audio::handle_set_volume(tx.clone(), parse(data)?).await,
        "increase_volume" => tools::audio::handle_increase_volume(tx.clone(), parse(data)?).await,
        "decrease_volume" => tools::audio::handle_decrease_volume(tx.clone(), parse(data)?).await,
        "is_muted" => tools::audio::handle_is_muted(tx.clone()).await,
        "set_mute" => tools::audio::handle_set_mute(tx.clone(), parse(data)?).await,

        //     MEDIA CONTROL
        "media_play" => tools::media::handle_media_play(tx.clone()).await,
        "media_pause" => tools::media::handle_media_pause(tx.clone()).await,
        "media_play_pause" => tools::media::handle_media_play_pause(tx.clone()).await,
        "media_stop" => tools::media::handle_media_stop(tx.clone()).await,
        "media_next_track" => tools::media::handle_media_next_track(tx.clone()).await,
        "media_previous_track" => tools::media::handle_media_previous_track(tx.clone()).await,
        "media_seek_forward" => {
            tools::media::handle_media_seek_forward(tx.clone(), parse(data)?).await
        }
        "media_seek_backward" => {
            tools::media::handle_media_seek_backward(tx.clone(), parse(data)?).await
        }
        "media_metadata" => tools::media::handle_media_metadata(tx.clone()).await,
        "media_position" => tools::media::handle_media_position(tx.clone()).await,
        "media_duration" => tools::media::handle_media_duration(tx.clone()).await,

        //     MUSIC INDEXER
        "search_music" => tools::music::handle_search_music(tx.clone(), parse(data)?).await,
        "play_music" => tools::music::handle_play_music(tx.clone(), parse(data)?).await,

        _ => Err(Error::UnknownTool(name).into()),
    }
}
