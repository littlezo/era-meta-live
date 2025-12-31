import { invoke } from '@tauri-apps/api/core';
import type { FollowedStreamer, LiveStreamInfo, LiveStatus } from '../common/types';

export async function refreshBilibiliFollowedStreamer(
  streamer: FollowedStreamer
): Promise<Partial<FollowedStreamer>> {
  try {
    const payloadData = { args: { room_id_str: streamer.id } };
    const data = await invoke<LiveStreamInfo>('fetch_bilibili_streamer_info', {
      payload: payloadData,
      cookie: null, // Optional: pass cookie if needed, but usually public info is enough
    });

    if (data && !data.error_message) {
      // Bilibili status: 1 = Live, 0/2 = Offline/Round
      const isLive = data.status === 1;
      const liveStatus: LiveStatus = isLive ? 'LIVE' : 'OFFLINE';

      return {
        isLive,
        liveStatus,
        nickname: data.anchor_name || streamer.nickname,
        roomTitle: data.title || streamer.roomTitle,
        avatarUrl: data.avatar || streamer.avatarUrl,
      };
    } else {
        if (data && data.error_message) {
             console.warn(
               `[BilibiliFollowHelper] Error fetching Bilibili room ${streamer.id}: ${data.error_message}`
             );
        }
      return { isLive: false, liveStatus: 'OFFLINE' };
    }
  } catch (e) {
    console.error(
      `[BilibiliFollowHelper] Failed to refresh Bilibili streamer ${streamer.id}:`,
      e
    );
    return { isLive: false, liveStatus: 'OFFLINE' };
  }
}
