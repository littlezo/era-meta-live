import { invoke } from '@tauri-apps/api/core';
import { listen, type Event as TauriEvent } from '@tauri-apps/api/event';
import { Ref } from 'vue';
import { Platform } from '../common/types';
import type { Message, MessageOverlayInstance, MessageRenderOptions, RustGetStreamUrlPayload } from '../../components/player/types';
import type { LiveStreamInfo } from '../common/types';
import { v4 as uuidv4 } from 'uuid';


export interface DouyinRustMessagePayload {
  room_id?: string; 
  user: string;      // Nickname from Rust's MessageFrontendPayload
  content: string;
  user_level: number; // from Rust's i64
  fans_club_level: number; // from Rust's i32
}

export async function fetchAndPrepareDouyinStreamConfig(roomId: string, quality: string = '原画'): Promise<{ 
  streamUrl: string | null;
  streamType: string | undefined; 
  title?: string | null; 
  anchorName?: string | null; 
  avatar?: string | null; 
  isLive: boolean; 
  initialError: string | null; // Made non-optional, will always be string or null
}> {
  if (!roomId) {
    return { streamUrl: null, streamType: undefined, title: null, anchorName: null, avatar: null, isLive: false, initialError: '房间ID未提供' };
  }

  try {
    const payloadData = { args: { room_id_str: roomId } };
    const backendQuality = normalizeDouyinQuality(quality);
    // 使用画质参数调用抖音画质切换API
    const result = await invoke<LiveStreamInfo>('get_douyin_live_stream_url_with_quality', { 
      payload: payloadData,
      quality: backendQuality 
    });

    if (result.error_message) {
      console.error(`[DouyinPlayerHelper] Error from backend for room ${roomId}: ${result.error_message}`);
      return {
        streamUrl: null,
        streamType: undefined,
        title: result.title,
        anchorName: result.anchor_name,
        avatar: result.avatar,
        isLive: result.status === 2,
        initialError: result.error_message, // string | null from Rust
      };
    }

    const streamAvailable = result.status === 2 && !!result.stream_url;
    let streamType: string | undefined = undefined;
    let uiMessage: string | null = null; 

    const rawStreamUrl = result.stream_url ?? null;
    const sanitizedStreamUrl = streamAvailable && rawStreamUrl ? enforceHttps(rawStreamUrl) : null;

    if (streamAvailable && rawStreamUrl) {
      if (rawStreamUrl.startsWith('http://127.0.0.1') && rawStreamUrl.endsWith('/live.flv')) {
        streamType = 'flv';
      } else if (rawStreamUrl.includes('pull-hls') || rawStreamUrl.endsWith('.m3u8')) {
        console.warn(`[DouyinPlayerHelper] Received HLS-like stream URL (${rawStreamUrl}), but expected flv. Overriding to flv.`);
        streamType = 'flv';
      } else if (rawStreamUrl.includes('pull-flv') || rawStreamUrl.includes('.flv')) {
        streamType = 'flv';
      } else {
        console.warn(`[DouyinPlayerHelper] Could not determine stream type for URL: ${rawStreamUrl}. Defaulting to flv.`);
        streamType = 'flv';
      }
      // uiMessage remains null if stream is available and no prior error.
    } else {
      if (result.status !== 2) {
        uiMessage = result.title ? `主播 ${result.anchor_name || ''} 未开播。` : '主播未开播或房间不存在。';
      } else {
        uiMessage = '主播在线，但获取直播流失败。';
      }
    }

    return {
      streamUrl: sanitizedStreamUrl,
      streamType: streamType,
      title: result.title,
      anchorName: result.anchor_name,
      avatar: result.avatar,
      isLive: streamAvailable,
      initialError: uiMessage, // uiMessage is definitely string or null here.
    };

  } catch (e: any) {
    console.error(`[DouyinPlayerHelper] Exception while fetching Douyin stream details for ${roomId}:`, e);
    return { 
        streamUrl: null, 
        streamType: undefined, 
        title: null, 
        anchorName: null, 
        avatar: null, 
        isLive: false, 
        initialError: e.message || '获取直播信息失败: 未知错误' // Ensure string here
    };
  }
}

function normalizeDouyinQuality(input: string): string {
  const upper = input.trim().toUpperCase();
  if (upper === 'OD' || upper === '原画') return 'OD';
  if (upper === 'BD' || upper === '高清') return 'BD';
  if (upper === 'UHD' || upper === '标清') return 'UHD';
  return 'OD';
}

export async function startDouyinMessageListener(
  roomId: string,
  messageOverlay: MessageOverlayInstance | null, // For emitting message to overlay
  messageMessagesRef: Ref<Message[]>, // For updating MessageList
  renderOptions?: MessageRenderOptions
): Promise<() => void> {
  
  const rustPayload: RustGetStreamUrlPayload = { 
    args: { room_id_str: roomId }, 
    platform: Platform.DOUYIN, 
  };
  await invoke('start_douyin_message_listener', { payload: rustPayload });
  
  const eventName = 'message';

  const unlisten = await listen<DouyinRustMessagePayload>(eventName, (event: TauriEvent<DouyinRustMessagePayload>) => {
    if (event.payload) {
      const rustP = event.payload;
      const frontendMessage: Message = {
        id: uuidv4(),
        nickname: rustP.user || '未知用户',
        content: rustP.content || '',
        level: String(rustP.user_level || 0),
        badgeLevel: rustP.fans_club_level > 0 ? String(rustP.fans_club_level) : undefined,
        room_id: rustP.room_id || roomId, // Ensure room_id is present
      };

      const shouldDisplay = renderOptions?.shouldDisplay ? renderOptions.shouldDisplay() : true;

      if (shouldDisplay && messageOverlay?.sendComment) {
        try {
          const commentOptions = renderOptions?.buildCommentOptions?.(frontendMessage) ?? {};
          const styleFromOptions = commentOptions.style ?? {};
          const preferredColor = styleFromOptions.color || frontendMessage.color || '#FFFFFF';
          messageOverlay.sendComment({
            id: frontendMessage.id,
            txt: frontendMessage.content,
            duration: commentOptions.duration ?? 12000,
            mode: commentOptions.mode ?? 'scroll',
            style: {
              ...styleFromOptions,
              color: preferredColor,
            },
          });
        } catch (emitError) {
          console.warn('[DouyinPlayerHelper] Failed emitting message comment:', emitError);
        }
      }
      messageMessagesRef.value.push(frontendMessage);
      if (messageMessagesRef.value.length > 200) { // Manage message array size
        messageMessagesRef.value.splice(0, messageMessagesRef.value.length - 200);
      }
    }
  });
  return unlisten;
}

export async function stopDouyinMessage(currentUnlistenFn: (() => void) | null): Promise<void> {
  if (currentUnlistenFn) {
    currentUnlistenFn();
  }
  try {
    const rustPayload: RustGetStreamUrlPayload = { 
      args: { room_id_str: "stop_listening" }, 
      platform: Platform.DOUYIN, 
    };
    await invoke('start_douyin_message_listener', { payload: rustPayload });
  } catch (error) {
    console.error('[DouyinPlayerHelper] Error stopping Douyin message listener:', error);
  }
}

function enforceHttps(url: string): string {
  if (!url) {
    return url;
  }
  if (url.startsWith('https://')) {
    return url;
  }
  if (url.startsWith('http://')) {
    return `https://${url.slice('http://'.length)}`;
  }
  return url;
}
