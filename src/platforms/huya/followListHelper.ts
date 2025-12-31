import { invoke } from '@tauri-apps/api/core';
import type { FollowedStreamer, LiveStatus } from '../common/types';

interface HuyaAnchorItem {
  room_id: string;
  avatar: string;
  user_name: string;
  live_status: boolean;
  title: string;
}

export async function refreshHuyaFollowedStreamer(
  streamer: FollowedStreamer
): Promise<Partial<FollowedStreamer>> {
  try {
    // Use search to get info by room ID
    const results = await invoke<HuyaAnchorItem[]>('search_huya_anchors', {
      keyword: streamer.id,
      page: 1,
    });

    if (results && results.length > 0) {
      // Find the exact match for room ID
      const match = results.find((item) => item.room_id === streamer.id);

      if (match) {
        const isLive = match.live_status;
        const liveStatus: LiveStatus = isLive ? 'LIVE' : 'OFFLINE';

        return {
          isLive,
          liveStatus,
          nickname: match.user_name || streamer.nickname,
          roomTitle: match.title || streamer.roomTitle,
          avatarUrl: match.avatar || streamer.avatarUrl,
        };
      }
    }
    
    // If no exact match found or empty results
    return { isLive: false, liveStatus: 'OFFLINE' };
  } catch (e) {
    console.error(
      `[HuyaFollowHelper] Failed to refresh Huya streamer ${streamer.id}:`,
      e
    );
    return { isLive: false, liveStatus: 'OFFLINE' };
  }
}
