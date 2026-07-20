use crate::{prelude::*, skills::SkillName, tools};
use json::from_value as parse;

#[derive(Deserialize)]
pub struct ListAction {
    #[serde(default)]
    pub skills: Vec<SkillName>,
}

/// API: Handles the tools list receiving
#[log(skip_all)]
pub async fn handle_tools_list(Json(ListAction { skills }): Json<ListAction>) -> Response {
    let tools = Settings::get().metadata.tools(skills);
    Response::ok().json(&tools)
}

/// API: Handles the agent tool call
#[log(skip_all, fields(tool = %name.0))]
pub async fn handle_tool_call(name: Paths<String>, payload: Json<JsonValue>) -> Response {
    info!("Initialized the `{}` tool handling", name.0);

    async fn tool_call(tx: Sender<Bytes>, name: String, payload: JsonValue) -> Result<()> {
        match name.as_ref() {
            //     SYSTEM MONITOR
            "get_system_info" => tools::info::handle_system_info(tx.clone()).await,
            "get_system_metrics" => tools::info::handle_system_metrics(tx.clone()).await,
            "get_devices_list" => tools::info::handle_devices_list(tx.clone()).await,

            //     SYSTEM THEME
            "set_theme" => tools::theme::handle_set_theme(tx.clone(), parse(payload)?).await,
            // TODO: "get_theme" => tools::handle_get_theme(tx.clone(), parse(payload)?).await,

            //     POWER MANAGEMENT
            "schedule_power" => {
                tools::power::handle_schedule_power(tx.clone(), parse(payload)?).await
            }
            "cancel_power" => tools::power::handle_cancel_power(tx.clone()).await,
            "get_power_status" => tools::power::handle_power_status(tx.clone()).await,

            //     AUDIO CONTROL
            "get_volume" => tools::audio::handle_get_volume(tx.clone()).await,
            "set_volume" => tools::audio::handle_set_volume(tx.clone(), parse(payload)?).await,
            "increase_volume" => {
                tools::audio::handle_increase_volume(tx.clone(), parse(payload)?).await
            }
            "decrease_volume" => {
                tools::audio::handle_decrease_volume(tx.clone(), parse(payload)?).await
            }
            "is_muted" => tools::audio::handle_is_muted(tx.clone()).await,
            "set_mute" => tools::audio::handle_set_mute(tx.clone(), parse(payload)?).await,

            //     MEDIA CONTROL
            "media_play" => tools::media::handle_media_play(tx.clone()).await,
            "media_pause" => tools::media::handle_media_pause(tx.clone()).await,
            "media_play_pause" => tools::media::handle_media_play_pause(tx.clone()).await,
            "media_stop" => tools::media::handle_media_stop(tx.clone()).await,
            "media_next_track" => tools::media::handle_media_next_track(tx.clone()).await,
            "media_previous_track" => tools::media::handle_media_previous_track(tx.clone()).await,
            "media_seek_forward" => {
                tools::media::handle_media_seek_forward(tx.clone(), parse(payload)?).await
            }
            "media_seek_backward" => {
                tools::media::handle_media_seek_backward(tx.clone(), parse(payload)?).await
            }
            "media_metadata" => tools::media::handle_media_metadata(tx.clone()).await,
            "media_position" => tools::media::handle_media_position(tx.clone()).await,
            "media_duration" => tools::media::handle_media_duration(tx.clone()).await,

            //     MUSIC INDEXER
            "search_music" => tools::music::handle_search_music(tx.clone(), parse(payload)?).await,
            "play_music" => tools::music::handle_play_music(tx.clone(), parse(payload)?).await,

            _ => Err(Error::UnknownTool(name).into()),
        }
    }

    Response::ok().stream(async move |tx| {
        if let Err(e) = tool_call(tx.clone(), name.0, payload.0).await {
            error!("{e}");
            tx.send(Event::error(e.to_string())).ok();
        }
    })
}
