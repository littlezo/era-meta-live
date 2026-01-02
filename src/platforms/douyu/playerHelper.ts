import { invoke } from '@tauri-apps/api/core';
import { listen, type Event as TauriEvent } from '@tauri-apps/api/event';
import { Ref } from 'vue';
import type { Message, MessageOverlayInstance, MessageRenderOptions } from '../../components/player/types';
import { v4 as uuidv4 } from 'uuid';

// 统一的 Rust 消息事件负载（与 Douyin/Huya 保持一致）
export interface UnifiedRustMessagePayload {
  room_id?: string;
  user: string;
  content: string;
  user_level: number;
  fans_club_level: number;
}

let douyuProxyActive = false;

export async function getDouyuStreamConfig(
  roomId: string,
  quality: string = '原画',
  line?: string | null,
): Promise<{ streamUrl: string, streamType: string | undefined }> {
  let finalStreamUrl: string | null = null;
  let streamType: string | undefined = undefined;
  const MAX_STREAM_FETCH_ATTEMPTS = 2;

  for (let attempt = 1; attempt <= MAX_STREAM_FETCH_ATTEMPTS; attempt++) {
    try {
      const streamUrl = await invoke<string>('get_stream_url_with_quality_cmd', {
        roomId: roomId,
        quality: quality,
        line: line ?? null,
      });
      
      if (streamUrl) {
        finalStreamUrl = enforceHttps(streamUrl);
        streamType = 'flv';
        break;
      } else {
        throw new Error('斗鱼直播流地址获取为空。');
      }
    } catch (e: any) {
      console.error(`[DouyuPlayerHelper] 获取斗鱼直播流失败 (尝试 ${attempt}/${MAX_STREAM_FETCH_ATTEMPTS}):`, e.message);
      const offlineOrInvalidRoomMessages = [
        '主播未开播',
        '房间不存在',
        'error: 1',
        'error: 102',
        'error code 1',
        'error code 102',
      ];

      const errorMessageLowerCase = e.message?.toLowerCase() || '';
      const isDefinitivelyOffline = offlineOrInvalidRoomMessages.some(msg => errorMessageLowerCase.includes(msg.toLowerCase()));

      if (isDefinitivelyOffline) {
        console.warn(`[DouyuPlayerHelper] Streamer for room ${roomId} is definitively offline or room is invalid. Aborting retries.`);
        throw e;
      }

      if (attempt === MAX_STREAM_FETCH_ATTEMPTS) {
        throw new Error(`获取斗鱼直播流失败 (尝试 ${MAX_STREAM_FETCH_ATTEMPTS} 次后): ${e.message}`);
      }
      await new Promise(resolve => setTimeout(resolve, 1000 * attempt)); 
    }
  }

  if (!finalStreamUrl) {
    throw new Error('未能获取有效的斗鱼直播流地址。');
  }

  try {
    await invoke('set_stream_url_cmd', { url: finalStreamUrl });
    const proxyUrl = await invoke<string>('start_proxy');
    douyuProxyActive = true;
    return { streamUrl: proxyUrl, streamType };
  } catch (e: any) {
    throw new Error(`设置斗鱼代理失败: ${e.message}`);
  }
}

export async function startDouyuMessageListener(
  roomId: string,
  messageOverlay: MessageOverlayInstance | null,
  messageMessagesRef: Ref<Message[]>,
  renderOptions?: MessageRenderOptions
): Promise<() => void> {

  await invoke('start_message_listener', { roomId });
  
  const eventName = 'message';

  const unlisten = await listen<UnifiedRustMessagePayload>(eventName, (event: TauriEvent<UnifiedRustMessagePayload>) => {
    if (event.payload) {
      const rustP = event.payload;

      // 仅处理当前 roomId 的消息，避免跨房间干扰
      if (rustP.room_id && rustP.room_id !== roomId) return;

      const frontendMessage: Message = {
        id: uuidv4(),
        nickname: rustP.user || '未知用户',
        content: rustP.content || '',
        level: String(rustP.user_level || 0),
        badgeLevel: rustP.fans_club_level > 0 ? String(rustP.fans_club_level) : undefined,
        room_id: rustP.room_id || roomId,
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
            duration: commentOptions.duration ?? 10000,
            mode: commentOptions.mode ?? 'scroll',
            style: {
              ...styleFromOptions,
              color: preferredColor,
            },
          });
        } catch (emitError) {
          console.warn('[DouyuPlayerHelper] Failed emitting message comment:', emitError);
        }
      }

      // 发送到消息列表
      messageMessagesRef.value.push(frontendMessage);
      if (messageMessagesRef.value.length > 200) {
        messageMessagesRef.value.splice(0, messageMessagesRef.value.length - 200);
      }
    }
  });
  
  return unlisten;
}

export async function stopDouyuMessage(roomId: string, currentUnlistenFn: (() => void) | null): Promise<void> {
  if (currentUnlistenFn) {
    currentUnlistenFn();
  }
  try {
    if (roomId) { 
        await invoke('stop_message_listener', { roomId: roomId });
    }
  } catch (error) {
    console.error('[DouyuPlayerHelper] Error invoking stop_message_listener for Douyu:', error);
  }
}

export async function stopDouyuProxy(): Promise<void> {
  if (!douyuProxyActive) {
    return;
  }
  try {
    await invoke('stop_proxy');
    douyuProxyActive = false;
  } catch (e) {
    console.error('[DouyuPlayerHelper] Error stopping proxy server:', e);
    douyuProxyActive = false;
  }
}

function enforceHttps(url: string): string {
  if (!url) return url;
  if (url.startsWith('https://')) return url;
  if (url.startsWith('http://')) {
    return `https://${url.slice('http://'.length)}`;
  }
  return url;
}
