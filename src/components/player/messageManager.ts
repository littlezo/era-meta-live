import type { Ref } from 'vue';

import { platformApi } from '../../platforms/common/platformApiService';
import type { SupportedPlatform } from '../../platforms/common/types';
// import { Platform as StreamingPlatform } from '../../platforms/common/types'; // Platform enum is no longer needed
// Platform-specific message listeners are now handled by SDK

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
  platform: SupportedPlatform,
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
    // Create render options for message display
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

    // Subscribe to message events using unified API
        const unlisten = await platformApi.subscribeToMessages((messageData) => {
            // 验证消息数据格式
            if (!messageData || typeof messageData !== 'object') {
                console.warn('[MessageManager] Invalid message data format:', messageData);
                return;
            }
            
            // 提取必要字段，添加默认值处理
            const messageId = messageData.id || `msg-${Date.now()}-${Math.random().toString(36).substr(2, 9)}`;
            const user = messageData.user || '未知用户';
            const content = messageData.content || '';
            const roomId = messageData.room_id || '';
            
            const newMessage: Message = {
                id: messageId,
                nickname: user,
                content: content,
                isSystem: false,
                type: (messageData.message_type as any) || 'normal',
                color: messageData.color || '#ffffff',
                room_id: roomId,
                level: messageData.user_level?.toString(),
                badgeLevel: messageData.fans_club_level?.toString() || messageData.fans_level?.toString(),
            };

            // Check if message should be displayed
            const shouldDisplay = renderOptions?.shouldDisplay ? renderOptions.shouldDisplay() : true;

            // Send message to overlay if available and should display
            if (shouldDisplay && messageOverlay?.sendComment) {
                try {
                    const commentOptions = renderOptions?.buildCommentOptions?.() ?? {};
                    const styleFromOptions = commentOptions.style ?? {};
                    const preferredColor = styleFromOptions.color || newMessage.color || '#FFFFFF';
                    
                    messageOverlay.sendComment({
                        id: newMessage.id,
                        txt: `${newMessage.nickname}: ${newMessage.content}`,
                        duration: commentOptions.duration ?? 10000,
                        mode: commentOptions.mode ?? 'scroll',
                        style: {
                            ...styleFromOptions,
                            color: preferredColor,
                        },
                    });
                } catch (emitError) {
                    console.warn('[Player] Failed emitting message comment:', emitError);
                }
            }

            // Add message to list regardless of display status
            ctx.messages.value.push(newMessage);
            
            // Limit message list to 200 messages
            if (ctx.messages.value.length > 200) {
                ctx.messages.value.shift();
            }
        }, roomId);

    // Create stop function - only unlisten, no backend listener management
    const stopFn = () => {
      unlisten();
    };

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

export const stopCurrentMessageListener = (
  ctx: MessageManagerContext,
  _platform?: SupportedPlatform,
) => {
  try {
    if (ctx.unlistenMessageFn.value) {
      // Call the stop function which will unlisten events
      ctx.unlistenMessageFn.value();
      ctx.unlistenMessageFn.value = null;
    }
  } catch (error) {
    console.error('[Player] Error stopping message listener:', error);
  }

  ctx.isMessageListenerActive.value = false;
};
