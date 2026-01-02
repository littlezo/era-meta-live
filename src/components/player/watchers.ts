import { watch, type ComputedRef, type Ref, type ShallowRef } from 'vue';

import { sanitizeMessageArea, sanitizeMessageOpacity } from './constants';
import type { MessageUserSettings } from './constants';
import type { MessageSettingsControl, MessageToggleControl, LineControl, QualityControl, RefreshControl, LineOption } from './plugins';
import { Platform as StreamingPlatform } from '../../platforms/common/types';
import type { Message, MessageOverlayInstance } from './types';

export interface PlayerProps {
  roomId: string | null;
  platform: StreamingPlatform;
  isFollowed?: boolean;
  streamUrl?: string | null;
  title?: string | null;
  anchorName?: string | null;
  avatar?: string | null;
  isLive?: boolean | null;
  initialError?: string | null;
  cookie?: string | null;
}

export interface PlayerWatcherContext {
  refreshControlPlugin: ShallowRef<RefreshControl | null>;
  isRefreshingStream: Ref<boolean>;
  qualityControlPlugin: ShallowRef<QualityControl | null>;
  isQualitySwitching: Ref<boolean>;
  lineControlPlugin: ShallowRef<LineControl | null>;
  isLineSwitching: Ref<boolean>;
  lineOptions: ComputedRef<LineOption[]>;
  currentLine: Ref<string | null>;
  getLineLabel: (key?: string | null) => string;
  persistLinePreference: (platform?: StreamingPlatform | null, lineKey?: string | null) => void;
  props: PlayerProps;
  resolveStoredLine: (platform?: StreamingPlatform | null) => string | null;
  isMessageEnabled: Ref<boolean>;
  messageTogglePlugin: ShallowRef<MessageToggleControl | null>;
  messageInstance: ShallowRef<MessageOverlayInstance | null>;
  messageSettingsPlugin: ShallowRef<MessageSettingsControl | null>;
  messageSettings: MessageUserSettings;
  applyMessageOverlayPreferences: (
    instance: MessageOverlayInstance | null,
    settings: MessageUserSettings,
    isEnabled: boolean,
    playerRoot?: HTMLElement | null,
  ) => void;
  syncMessageEnabledState: (
    instance: MessageOverlayInstance | null,
    settings: MessageUserSettings,
    isEnabled: boolean,
    playerRoot?: HTMLElement | null,
  ) => void;
  persistCurrentMessagePreferences: () => void;
  currentQuality: Ref<string>;
  initializeQualityPreference: () => void;
  initializePlayerAndStream: (
    roomId: string,
    platform: StreamingPlatform,
    streamUrl?: string | null,
    isRefresh?: boolean,
    oldRoomIdForCleanup?: string | null,
    oldPlatformForCleanup?: StreamingPlatform | null,
  ) => Promise<void>;
  stopCurrentMessageListener: (platform?: StreamingPlatform) => Promise<void>;
  stopDouyuProxy: () => Promise<void>;
  destroyPlayerInstance: () => void;
  isLoadingStream: Ref<boolean>;
  messageMessages: Ref<Message[]>;
  streamError: Ref<string | null>;
  isOfflineError: Ref<boolean>;
  playerTitle: Ref<string | null | undefined>;
  playerAnchorName: Ref<string | null | undefined>;
  playerAvatar: Ref<string | null | undefined>;
  playerIsLive: Ref<boolean | null | undefined>;
  playerRoot: () => HTMLElement | null | undefined;
}

export const registerPlayerWatchers = (ctx: PlayerWatcherContext) => {
  const {
    refreshControlPlugin,
    isRefreshingStream,
    qualityControlPlugin,
    isQualitySwitching,
    lineControlPlugin,
    isLineSwitching,
    lineOptions,
    currentLine,
    getLineLabel,
    persistLinePreference,
    props,
    resolveStoredLine,
    isMessageEnabled,
    messageTogglePlugin,
    messageInstance,
    messageSettingsPlugin,
    messageSettings,
    applyMessageOverlayPreferences,
    syncMessageEnabledState,
    persistCurrentMessagePreferences,
    currentQuality,
    initializeQualityPreference,
    initializePlayerAndStream,
    stopCurrentMessageListener,
    stopDouyuProxy,
    destroyPlayerInstance,
    isLoadingStream,
    messageMessages,
    streamError,
    isOfflineError,
    playerTitle,
    playerAnchorName,
    playerAvatar,
    playerIsLive,
    playerRoot,
  } = ctx;

  watch(isRefreshingStream, (isLoading) => {
    refreshControlPlugin.value?.setLoading(isLoading);
  });

  watch(refreshControlPlugin, (plugin) => {
    plugin?.setLoading(isRefreshingStream.value);
  });

  watch(isQualitySwitching, (isSwitching) => {
    qualityControlPlugin.value?.setSwitching(isSwitching);
  });

  watch(qualityControlPlugin, (plugin) => {
    plugin?.setSwitching(isQualitySwitching.value);
  });

  watch(isLineSwitching, (isSwitching) => {
    lineControlPlugin.value?.setSwitching(isSwitching);
  });

  watch(lineControlPlugin, (plugin) => {
    if (!plugin) {
      return;
    }
    plugin.setOptions(lineOptions.value);
    plugin.updateLabel(getLineLabel(currentLine.value));
    plugin.setSwitching(isLineSwitching.value);
  });

  watch(
    () => props.platform,
    (platform, previous) => {
      if (platform !== previous) {
        currentLine.value = resolveStoredLine(platform);
        isLineSwitching.value = false;
      }
    },
  );

  watch(
    lineOptions,
    (options) => {
      if (!options.length) {
        currentLine.value = null;
      } else if (!options.some((option) => option.key === currentLine.value)) {
        currentLine.value = options[0]?.key ?? null;
      }
      lineControlPlugin.value?.setOptions(options);
      lineControlPlugin.value?.updateLabel(getLineLabel(currentLine.value));
    },
    { immediate: true },
  );

  watch(currentLine, (line) => {
    if (line) {
      persistLinePreference(props.platform, line);
    }
    lineControlPlugin.value?.updateLabel(getLineLabel(line));
  });

  watch(isMessageEnabled, (enabled) => {
    messageTogglePlugin.value?.setState(enabled);
    syncMessageEnabledState(messageInstance.value, messageSettings, enabled, playerRoot());
    persistCurrentMessagePreferences();
  });

  watch(messageTogglePlugin, (plugin) => {
    plugin?.setState(isMessageEnabled.value);
  });

  watch(messageSettingsPlugin, (plugin) => {
    if (!plugin) {
      return;
    }
    plugin.setSettings({
      color: messageSettings.color,
      strokeColor: messageSettings.strokeColor,
      fontSize: messageSettings.fontSize,
      duration: messageSettings.duration,
      area: sanitizeMessageArea(messageSettings.area),
      mode: messageSettings.mode,
      opacity: sanitizeMessageOpacity(messageSettings.opacity),
    });
  });

  watch(() => messageSettings.color, (color) => {
    messageSettingsPlugin.value?.setSettings({ color });
    persistCurrentMessagePreferences();
  });

  watch(() => messageSettings.strokeColor, (strokeColor) => {
    messageSettingsPlugin.value?.setSettings({ strokeColor });
    applyMessageOverlayPreferences(messageInstance.value, messageSettings, isMessageEnabled.value, playerRoot());
    persistCurrentMessagePreferences();
  });

  watch(() => messageSettings.fontSize, (fontSize) => {
    messageSettingsPlugin.value?.setSettings({ fontSize });
    applyMessageOverlayPreferences(messageInstance.value, messageSettings, isMessageEnabled.value, playerRoot());
    persistCurrentMessagePreferences();
  });

  watch(() => messageSettings.duration, (duration) => {
    messageSettingsPlugin.value?.setSettings({ duration });
    applyMessageOverlayPreferences(messageInstance.value, messageSettings, isMessageEnabled.value, playerRoot());
    persistCurrentMessagePreferences();
  });

  watch(() => messageSettings.area, (area) => {
    const normalizedArea = sanitizeMessageArea(area);
    if (normalizedArea !== area) {
      messageSettings.area = normalizedArea;
      return;
    }
    messageSettingsPlugin.value?.setSettings({ area: normalizedArea });
    applyMessageOverlayPreferences(messageInstance.value, messageSettings, isMessageEnabled.value, playerRoot());
    persistCurrentMessagePreferences();
  });

  watch(() => messageSettings.opacity, (opacity) => {
    const normalizedOpacity = sanitizeMessageOpacity(opacity);
    if (normalizedOpacity !== opacity) {
      messageSettings.opacity = normalizedOpacity;
      return;
    }
    messageSettingsPlugin.value?.setSettings({ opacity: normalizedOpacity });
    applyMessageOverlayPreferences(messageInstance.value, messageSettings, isMessageEnabled.value, playerRoot());
    persistCurrentMessagePreferences();
  });

  watch(messageInstance, (instance) => {
    applyMessageOverlayPreferences(instance, messageSettings, isMessageEnabled.value, playerRoot());
    syncMessageEnabledState(instance, messageSettings, isMessageEnabled.value, playerRoot());
  });

  watch(currentQuality, (quality) => {
    qualityControlPlugin.value?.updateLabel(quality);
  });

  watch(
    [
      () => props.roomId,
      () => props.platform,
      () => props.streamUrl,
      () => props.avatar,
      () => props.title,
      () => props.anchorName,
      () => props.isLive,
    ],
    async (
      [newRoomId, newPlatform, newStreamUrl, newAvatar, newTitle, newAnchorName, newIsLive],
      [oldRoomId, oldPlatform, oldStreamUrl],
    ) => {
      if (newPlatform === StreamingPlatform.DOUYU) {
        playerTitle.value = newTitle;
        playerAnchorName.value = newAnchorName;
        playerAvatar.value = newAvatar;
        if (newIsLive !== undefined) {
          playerIsLive.value = newIsLive;
        }
      }

      if (newRoomId && newPlatform) {
        if (!(props.initialError && props.initialError.includes('主播未开播'))) {
          isOfflineError.value = false;
        }

        const isInitialCall = oldRoomId === undefined && oldPlatform === undefined;
        const hasSwitchedStream = newRoomId !== oldRoomId || newPlatform !== oldPlatform;
        const douyinStreamUrlChanged = newPlatform === StreamingPlatform.DOUYIN && newStreamUrl !== oldStreamUrl;

        const needsReInit = hasSwitchedStream || isInitialCall || douyinStreamUrlChanged;

        if (needsReInit) {
          initializeQualityPreference();
          initializePlayerAndStream(newRoomId, newPlatform, newStreamUrl, false, oldRoomId, oldPlatform);
        }
      } else if (!newRoomId) {
        if (oldRoomId && oldPlatform !== null && oldPlatform !== undefined) {
          await stopCurrentMessageListener(oldPlatform);
          if (oldPlatform === StreamingPlatform.DOUYU) {
            await stopDouyuProxy();
          }
        } else {
          await stopCurrentMessageListener();
        }

        destroyPlayerInstance();

        isLoadingStream.value = false;
        messageMessages.value = [];
        streamError.value = null;
        isOfflineError.value = false;
      }
      if (!props.roomId || props.platform == null) {
        if (props.initialError) {
          if (props.initialError.includes('主播未开播')) {
            streamError.value = props.initialError;
            isOfflineError.value = true;
          } else {
            streamError.value = props.initialError;
            isOfflineError.value = false;
          }
        }
        isLoadingStream.value = false;
      }
    },
    { immediate: true },
  );
};
