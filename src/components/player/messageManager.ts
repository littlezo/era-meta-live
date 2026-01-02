import type { Ref } from 'vue';

import { Platform as StreamingPlatform } from '../../platforms/common/types';
import { startBilibiliMessageListener, stopBilibiliMessage } from '../../platforms/bilibili/playerHelper';
import { startDouyinMessageListener, stopDouyinMessage } from '../../platforms/douyin/playerHelper';
import { startDouyuMessageListener } from '../../platforms/douyu/playerHelper';
import { startHuyaMessageListener, stopHuyaMessage } from '../../platforms/huya/playerHelper';

import type { MessageUserSettings } from './constants';
import type { PlayerProps } from './watchers';
import type { Message, MessageOverlayInstance } from './types';

export interface MessageManagerContext {
  messages: Ref<Message[]>;
  isMessageEnabled: Ref<boolean>;
  messageSettings: MessageUserSettings;
  isMessageListenerActive: Ref<boolean>;
  unlistenMessageFn: Ref<(() => void) | null>;
  props: PlayerProps;
}

export const startCurrentMessageListener = async (
  ctx: MessageManagerContext,
  platform: StreamingPlatform,
  roomId: string,
  messageOverlay: MessageOverlayInstance | null,
) => {
  if (!roomId || ctx.isMessageListenerActive.value) {
    return;
  }

  ctx.isMessageListenerActive.value = true;
  if (!messageOverlay) {
    console.warn('[Player] Message overlay instance missing, incoming messages will not render on video but list will update.');
  }

  try {
    const renderOptions = {
      shouldDisplay: () => ctx.isMessageEnabled.value,
      buildCommentOptions: () => ({
        duration: ctx.messageSettings.duration,
        mode: ctx.messageSettings.mode,
        style: {
          color: ctx.messageSettings.color,
          fontSize: ctx.messageSettings.fontSize,
          '--message-stroke-color': ctx.messageSettings.strokeColor,
        },
      }),
    };
    let stopFn: (() => void) | null = null;
    if (platform === StreamingPlatform.DOUYU) {
      stopFn = await startDouyuMessageListener(roomId, messageOverlay, ctx.messages, renderOptions);
    } else if (platform === StreamingPlatform.DOUYIN) {
      stopFn = await startDouyinMessageListener(roomId, messageOverlay, ctx.messages, renderOptions);
    } else if (platform === StreamingPlatform.HUYA) {
      stopFn = await startHuyaMessageListener(roomId, messageOverlay, ctx.messages, renderOptions);
    } else if (platform === StreamingPlatform.BILIBILI) {
      stopFn = await startBilibiliMessageListener(roomId, messageOverlay, ctx.messages, ctx.props.cookie || undefined, renderOptions);
    }

    if (stopFn) {
      ctx.unlistenMessageFn.value = stopFn;
      const successMessage: Message = {
        id: `system-conn-${Date.now()}`,
        nickname: '系统消息',
        content: '消息连接成功！',
        isSystem: true,
        type: 'success',
        color: '#28a745',
      };
      ctx.messages.value.push(successMessage);
    } else {
      console.warn(`[Player] Message listener for ${platform}/${roomId} did not return a stop function.`);
      ctx.isMessageListenerActive.value = false;
    }
  } catch (error) {
    console.error(`[Player] Failed to start message listener for ${platform}/${roomId}:`, error);
    ctx.isMessageListenerActive.value = false;

    const errorMessage: Message = {
      id: `system-err-${Date.now()}`,
      nickname: '系统消息',
      content: '消息连接失败，请尝试刷新播放器。',
      isSystem: true,
      type: 'error',
      color: '#dc3545',
    };
    ctx.messages.value.push(errorMessage);
  }
};

export const stopCurrentMessageListener = async (
  ctx: MessageManagerContext,
  platform?: StreamingPlatform,
) => {
  if (platform) {
    if (platform === StreamingPlatform.DOUYIN) {
      await stopDouyinMessage(ctx.unlistenMessageFn.value);
    } else if (platform === StreamingPlatform.HUYA) {
      await stopHuyaMessage(ctx.unlistenMessageFn.value);
    } else if (platform === StreamingPlatform.BILIBILI) {
      await stopBilibiliMessage(ctx.unlistenMessageFn.value);
    }
    if (ctx.unlistenMessageFn.value) {
      ctx.unlistenMessageFn.value();
      ctx.unlistenMessageFn.value = null;
    }
  } else if (ctx.unlistenMessageFn.value) {
    console.warn('[Player] stopCurrentMessageListener called without platform, but a global unlistenMessageFn exists. Calling it now.');
    try {
      ctx.unlistenMessageFn.value();
      ctx.unlistenMessageFn.value = null;
    } catch (error) {
      console.error('[Player] Error executing fallback unlistenMessageFn:', error);
      ctx.unlistenMessageFn.value = null;
    }
  }

  ctx.isMessageListenerActive.value = false;
};
