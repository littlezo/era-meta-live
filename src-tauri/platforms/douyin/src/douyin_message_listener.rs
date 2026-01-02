use crate::web_api::normalize_douyin_live_id;
use tauri::Emitter;
use tokio::sync::mpsc as tokio_mpsc;

#[tauri::command]
pub async fn start_douyin_message_listener(
    payload: shared::GetStreamUrlPayload,
    app_handle: tauri::AppHandle,
    state: tauri::State<'_, shared::DouyinMessageState>,
) -> Result<(), String> {
    let room_id_or_url = payload.args.room_id_str;
    println!(
        "[Douyin Message] Received request for room_id_or_url: {}",
        room_id_or_url
    );

    let previous_tx = {
        let mut lock = state.inner().0.lock().unwrap();
        lock.take()
    };

    if let Some(tx) = previous_tx {
        println!("[Douyin Message] Sending shutdown to previous Douyin listener task.");
        let _ = tx.send(()).await;
    }

    if room_id_or_url == "stop_listening" {
        println!(
            "[Douyin Message] Received stop_listening signal. Listener will not be restarted."
        );
        return Ok(());
    }

    let normalized_room_id = normalize_douyin_live_id(&room_id_or_url);

    let (tx_shutdown, mut rx_shutdown) = tokio_mpsc::channel::<()>(1);
    {
        let mut lock = state.inner().0.lock().unwrap();
        *lock = Some(tx_shutdown);
    }

    let app_handle_clone = app_handle.clone();
    let room_id_str_clone = normalized_room_id.clone();

    tokio::spawn(async move {
        println!(
            "[Douyin Message] Spawning listener for room: {}",
            room_id_str_clone
        );

        let task_result = {
            let mut attempt: u32 = 1;
            loop {
                let attempt_result = async {
                    let mut fetcher = crate::message::web_fetcher::DouyinLiveWebFetcher::new(&room_id_str_clone)?;
                    fetcher
                        .fetch_room_details()
                        .await
                        .map_err(|e| format!("Failed to fetch room details: {}", e))?;

                    let actual_room_id = fetcher.get_room_id().await?;
                    let cookie_header = fetcher.get_dy_cookie().await?;
                    let user_unique_id = fetcher.get_user_unique_id().await?;
                    println!(
                        "[Douyin Message] Using: room_id={}, user_unique_id={}",
                        actual_room_id, user_unique_id
                    );

                    let (read_stream, ack_tx) = crate::message::websocket_connection::connect_and_manage_websocket(
                        &fetcher,
                        &actual_room_id,
                        &cookie_header,
                        &user_unique_id,
                    )
                    .await?;

                    println!(
                        "[Douyin Message] WebSocket connected for room: {}",
                        actual_room_id
                    );

                    tokio::select! {
                        res = crate::message::message_handler::handle_received_messages(
                            read_stream,
                            ack_tx,
                            app_handle_clone.clone(),
                            actual_room_id.clone()
                        ) => {
                            if let Err(e) = res {
                                return Err(e);
                            }
                        }
                        _ = rx_shutdown.recv() => {
                            println!(
                                "[Douyin Message] Received shutdown signal for room {}.",
                                actual_room_id
                            );
                        }
                    }

                    Ok::<(), Box<dyn std::error::Error + Send + Sync>>(())
                }
                .await;

                match attempt_result {
                    Ok(_) => break Ok(()),
                    Err(e) => {
                        if attempt >= 2 {
                            eprintln!("[Douyin Message] Listener connect/fetch failed after {} attempts: {}", attempt, e);
                            break Err(e);
                        } else {
                            eprintln!(
                                "[Douyin Message WARN] Attempt {} failed: {}. Retrying...",
                                attempt, e
                            );
                            attempt += 1;
                            continue;
                        }
                    }
                }
            }
        };

        if let Err(e) = task_result {
            eprintln!(
                "[Douyin Message] Listener task for room {} critically failed: {}",
                room_id_str_clone, e
            );
            let error_payload = shared::MessageFrontendPayload {
                room_id: room_id_str_clone.clone(),
                user: "系统消息".to_string(),
                content: format!("弹幕连接发生错误: {}", e),
                user_level: 0,
                fans_club_level: 0,
            };
            if let Err(emit_err) = app_handle.emit("message", error_payload) {
                eprintln!(
                    "[Douyin Message] Failed to emit error event to frontend: {}",
                    emit_err
                );
            }
        } else {
            println!(
                "[Douyin Message] Listener task for room {} completed.",
                room_id_str_clone
            );
        }
    });
    Ok(())
}
