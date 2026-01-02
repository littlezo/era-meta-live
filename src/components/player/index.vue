<template>
  <div class="player-page" :class="{ 'web-fs': isInWebFullscreen || isInNativePlayerFullscreen }">
    <button v-if="!isInWebFullscreen" @click="$emit('close-player')" class="player-close-btn" title="关闭播放器">
      <svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
        <line x1="18" y1="6" x2="6" y2="18"></line>
        <line x1="6" y1="6" x2="18" y2="18"></line>
      </svg>
    </button>

    <div class="player-layout">
      <div class="main-content">
        <div v-if="!roomId" class="empty-player">
          <div class="empty-icon">
            <svg width="64" height="64" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
               <circle cx="12" cy="12" r="10"></circle>
               <line x1="12" y1="16" x2="12" y2="12"></line>
               <line x1="12" y1="8" x2="12.01" y2="8"></line>
            </svg>
          </div>
          <h3>未选择直播间</h3>
          <p>请从首页选择一个直播间开始观看。</p>
        </div>
        <div v-else-if="isLoadingStream" class="loading-player">
          <LoadingDots />
        </div>
        <div v-else-if="isOfflineError" class="offline-player">
          <!-- Display StreamerInfo if room details are available -->
          <StreamerInfo 
            v-if="props.roomId && props.platform"
            :room-id="props.roomId"
            :platform="props.platform"
            :title="playerTitle"
            :anchor-name="playerAnchorName"
            :avatar="playerAvatar"
            :is-live="false"
            :is-followed="props.isFollowed"
            @follow="$emit('follow', $event)"
            @unfollow="$emit('unfollow', $event)"
            class="streamer-info-offline"
          />
          <div class="offline-message">
            <div class="offline-icon">
              <svg xmlns="http://www.w3.org/2000/svg" width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
                <path d="M16 16.427A4.002 4.002 0 0 0 12.005 20a4 4 0 0 0-3.995-3.573M12 12V2M8.5 7L7 5.5M15.5 7l1.5-1.5M5.562 10.223l-1.842.511M18.438 10.223l1.842.511M12 2a3.5 3.5 0 0 1 3.5 3.5V12H8.5V5.5A3.5 3.5 0 0 1 12 2z"/>
                <line x1="1" y1="1" x2="23" y2="23" stroke-width="2"></line> 
              </svg>
            </div>
            <h3>😴 获取直播流失败了</h3>
            <p>主播当前未开播，请稍后再来。</p>
            <button @click="retryInitialization" class="retry-btn">再试一次</button>
          </div>
        </div>
        <div v-else-if="streamError && !isOfflineError" class="error-player">
          <div class="error-icon">
             <svg width="64" height="64" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
              <circle cx="12" cy="12" r="10"></circle>
              <line x1="12" y1="8" x2="12" y2="12"></line>
              <line x1="12" y1="16" x2="12.01" y2="16"></line>
            </svg>
          </div>
          <h3>加载失败</h3>
          <p>{{ streamError }}</p>
          <button @click="retryInitialization" class="retry-btn">再试一次</button>
        </div>
        <div v-else class="player-container">
          <StreamerInfo
            v-if="props.roomId"
            :room-id="props.roomId"
            :platform="props.platform"
            :title="playerTitle"
            :anchor-name="playerAnchorName"
            :avatar="playerAvatar"
            :is-followed="props.isFollowed"
            :is-live="playerIsLive"
            @follow="$emit('follow', $event)"
            @unfollow="$emit('unfollow', $event)"
            class="streamer-info"
            v-show="!isInWebFullscreen"
            :class="{'hidden-panel': isInWebFullscreen}"
          />
          <div class="video-container">
            <div ref="playerContainerRef" class="video-player"></div>
          </div>
        </div>
      </div>

      <MessageList 
        v-if="roomId && !isLoadingStream && !streamError" 
        :room-id="props.roomId"
        :messages="messageMessages"
        v-show="!isFullScreen" 
        class="message-panel" 
        :class="{'hidden-panel': isFullScreen}"
      />
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, reactive, ref, shallowRef } from 'vue';
import Player from 'xgplayer';
import FlvPlugin from 'xgplayer-flv';
import HlsPlugin from 'xgplayer-hls.js';
import { POSITIONS } from 'xgplayer/es/plugin/plugin.js';
import 'xgplayer/dist/index.min.css';

import './player.css';

import { Platform as StreamingPlatform } from '../../platforms/common/types';
import type { Message, MessageOverlayInstance } from './types';
import {
  applyMessageFontFamilyForOS,
  ICONS,
  loadMessagePreferences,
  loadStoredVolume,
  persistMessagePreferences,
  sanitizeMessageArea,
  sanitizeMessageOpacity,
  type MessageUserSettings,
} from './constants';
import {
  MessageSettingsControl,
  MessageToggleControl,
  LineControl,
  QualityControl,
  RefreshControl,
  VolumeControl,
} from './plugins';
import { arrangeControlClusters } from './controlLayout';
import { applyMessageOverlayPreferences, createMessageOverlay, ensureMessageOverlayHost, syncMessageEnabledState } from './messageOverlay';
import { registerPlayerWatchers, type PlayerProps } from './watchers';
import { startCurrentMessageListener as startMessageListener, stopCurrentMessageListener as stopMessageListener } from './messageManager';
import { getLineLabel, getLineOptionsForPlatform, persistLinePreference, resolveCurrentLineFor, resolveStoredLine } from './lineOptions';

// Platform-specific player helpers
import { getDouyuStreamConfig, stopDouyuProxy } from '../../platforms/douyu/playerHelper';
import { fetchAndPrepareDouyinStreamConfig } from '../../platforms/douyin/playerHelper';
import { getHuyaStreamConfig } from '../../platforms/huya/playerHelper';
import { getBilibiliStreamConfig } from '../../platforms/bilibili/playerHelper';

import StreamerInfo from '../StreamerInfo/index.vue';
import MessageList from '../MessageList/index.vue';
import LoadingDots from '../Common/LoadingDots.vue';

import { invoke } from '@tauri-apps/api/core';
import { useImageProxy } from '../FollowsList/useProxy';

// Ensure image proxy helpers are available in this component
const { ensureProxyStarted, proxify } = useImageProxy();

const props = defineProps<PlayerProps>();

const emit = defineEmits<{
  (e: 'follow', streamer: any): void;
  (e: 'unfollow', roomId: string): void;
  (e: 'close-player'): void;
  (e: 'fullscreen-change', isFullscreen: boolean): void;
  (e: 'request-refresh-details'): void;
  (e: 'request-player-reload'): void;
}>();

const playerContainerRef = ref<HTMLDivElement | null>(null);
const playerInstance = shallowRef<Player | null>(null);
const refreshControlPlugin = shallowRef<RefreshControl | null>(null);
const qualityControlPlugin = shallowRef<QualityControl | null>(null);
const lineControlPlugin = shallowRef<LineControl | null>(null);
const messageTogglePlugin = shallowRef<MessageToggleControl | null>(null);
const messageSettingsPlugin = shallowRef<MessageSettingsControl | null>(null);
const volumeControlPlugin = shallowRef<VolumeControl | null>(null);
const messageInstance = shallowRef<MessageOverlayInstance | null>(null);
const messageMessages = ref<Message[]>([]);
const isMessageListenerActive = ref(false); // Tracks if a message listener is supposed to be running
const unlistenMessageFn = ref<(() => void) | null>(null);

const isLoadingStream = ref(true);
const streamError = ref<string | null>(null);
const isOfflineError = ref(false); // Added to track '主播未开播' state

// Reactive state for streamer info, initialized by props, potentially updated by internal fetches (for Douyin)
const playerTitle = ref(props.title);
const playerAnchorName = ref(props.anchorName);
const playerAvatar = ref(props.avatar);
const playerIsLive = ref(props.isLive);

const isInNativePlayerFullscreen = ref(false); // New: Tracks Artplayer element's native fullscreen
const isInWebFullscreen = ref(false);
const isFullScreen = ref(false); // True if EITHER native player OR web fullscreen is active

const isMessageEnabled = ref(true);
const messageSettings = reactive<MessageUserSettings>({
  color: '#ffffff',
  strokeColor: '#444444',
  fontSize: '20px',
  duration: 10000,
  area: 0.5,
  mode: 'scroll',
  opacity: 1,
});

const storedMessagePreferences = loadMessagePreferences();
if (storedMessagePreferences) {
  isMessageEnabled.value = storedMessagePreferences.enabled;
  Object.assign(messageSettings, storedMessagePreferences.settings);
}

// OS specific states
const osName = ref<string>('');

// 画质切换相关
const qualityOptions = ['原画', '高清', '标清'] as const;

const resolveStoredQuality = (platform?: StreamingPlatform | null): string => {
  if (!platform) {
    return '原画';
  }
  if (typeof window === 'undefined') {
    return '原画';
  }
  try {
    const saved = window.localStorage.getItem(`${platform}_preferred_quality`);
    if (saved && qualityOptions.includes(saved as (typeof qualityOptions)[number])) {
      return saved;
    }
  } catch (error) {
    console.warn('[Player] Failed to read stored quality preference:', error);
  }
  return '原画';
};

const currentQuality = ref<string>(resolveStoredQuality(props.platform));
const isQualitySwitching = ref(false);
const isRefreshingStream = ref(false);
const isLineSwitching = ref(false);

const currentLine = ref<string | null>(resolveStoredLine(props.platform));
const lineOptions = computed(() => getLineOptionsForPlatform(props.platform));
const getCurrentLineLabel = (key?: string | null) => getLineLabel(lineOptions.value, key);

function resetFullscreenState() {
  isInNativePlayerFullscreen.value = false;
  isInWebFullscreen.value = false;
  isFullScreen.value = false;
  try {
    document.documentElement.classList.remove('web-fs-active');
  } catch (error) {
    console.warn('[Player] Failed to reset web fullscreen flag:', error);
  }
}

function updateFullscreenFlag() {
  isFullScreen.value = isInNativePlayerFullscreen.value || isInWebFullscreen.value;
  emit('fullscreen-change', isFullScreen.value);
}

function destroyPlayerInstance() {
  const player = playerInstance.value;
  if (player) {
    try {
      player.destroy();
    } catch (error) {
      console.error('[Player] Error destroying xgplayer instance:', error);
    }
    const overlayHost = player.root?.querySelector('.player-message-overlay') as HTMLElement | null;
    overlayHost?.remove();
  }
  playerInstance.value = null;

  const message = messageInstance.value;
  if (message) {
    try {
      message.stop?.();
    } catch (error) {
      console.error('[Player] Error stopping message overlay:', error);
    }
    messageInstance.value = null;
  }

  refreshControlPlugin.value = null;
  qualityControlPlugin.value = null;
  lineControlPlugin.value = null;
  messageTogglePlugin.value = null;
  messageSettingsPlugin.value = null;
  volumeControlPlugin.value = null;

  resetFullscreenState();
}

async function mountXgPlayer(
  streamUrl: string,
  platformCode: StreamingPlatform,
  roomId: string,
  streamType?: string | null,
) {
  await nextTick();

  if (!playerContainerRef.value) {
    streamError.value = '播放器容器初始化失败。';
    return;
  }

  playerContainerRef.value.innerHTML = '';

  const playbackType = streamType === 'hls' ? 'hls' : 'flv';
  const isHlsPlayback = playbackType === 'hls';

  const playerOptions: Record<string, any> = {
    el: playerContainerRef.value,
    url: streamUrl,
    isLive: true,
    autoplay: true,
    playsinline: true,
    lang: 'zh-cn',
    width: '100%',
    height: '100%',
    videoFillMode: 'contain',
    closeVideoClick: true,
    closeVideoTouch: true,
    keyShortcut: true,
    volume: false as unknown as number,
    pip: {
      position: POSITIONS.CONTROLS_RIGHT,
      index: 3,
      showIcon: true,
    },
    cssFullscreen: {
      index: 2,
    },
    playbackRate: false,
    controls: {
      mode: 'normal',
    },
    icons: {
      play: ICONS.play,
      pause: ICONS.pause,
      fullscreen: ICONS.maximize2,
      exitFullscreen: ICONS.minimize2,
      cssFullscreen: ICONS.fullscreen,
      exitCssFullscreen: ICONS.minimize2,
      pipIcon: ICONS.pictureInPicture2,
      pipIconExit: ICONS.pictureInPicture2,
    },
  };

  if (isHlsPlayback) {
    const hlsFetchOptions: RequestInit = {
      referrer: 'https://live.bilibili.com/',
      referrerPolicy: 'no-referrer-when-downgrade',
      credentials: 'omit',
      mode: 'cors',
    };

    playerOptions.plugins = [HlsPlugin];
    playerOptions.useHlsPlugin = true;
    playerOptions.hls = {
      isLive: true,
      retryCount: 3,
      retryDelay: 2000,
      enableWorker: true,
      withCredentials: false,
      lowLatencyMode: false,
      fetchOptions: hlsFetchOptions,
      xhrSetup: (xhr: XMLHttpRequest) => {
        try {
          xhr.withCredentials = false;
          xhr.setRequestHeader('Referer', 'https://live.bilibili.com/');
          xhr.setRequestHeader('Origin', 'https://live.bilibili.com');
        } catch (headerError) {
          console.warn('[Player] Failed to attach Bilibili HLS headers:', headerError);
        }
      },
    };
  } else {
    playerOptions.plugins = [FlvPlugin];
    playerOptions.flv = {
      isLive: true,
      cors: true,
      autoCleanupSourceBuffer: true,
      enableWorker: true,
      stashInitialSize: 128,
      lazyLoad: true,
      lazyLoadMaxDuration: 30,
      deferLoadAfterSourceOpen: true,
    };
  }

  const player = new Player(playerOptions);

  playerInstance.value = player;
  const storedPlayerVolume = loadStoredVolume();
  if (storedPlayerVolume !== null) {
    player.volume = storedPlayerVolume;
    player.muted = storedPlayerVolume === 0 ? true : player.muted;
  }

  const lineOptionsForPlatform = lineOptions.value.map((option) => ({ ...option }));

  refreshControlPlugin.value = player.registerPlugin(RefreshControl, {
    position: POSITIONS.CONTROLS_LEFT,
    index: 2,
    onClick: () => {
      void reloadCurrentStream('refresh');
    },
  }) as RefreshControl;

  volumeControlPlugin.value = player.registerPlugin(VolumeControl, {
    position: POSITIONS.CONTROLS_LEFT,
    index: 3,
  }) as VolumeControl;

  messageTogglePlugin.value = player.registerPlugin(MessageToggleControl, {
    position: POSITIONS.CONTROLS_RIGHT,
    index: 4,
    getState: () => isMessageEnabled.value,
    onToggle: (enabled: boolean) => {
      isMessageEnabled.value = enabled;
    },
  }) as MessageToggleControl;

  messageSettingsPlugin.value = player.registerPlugin(MessageSettingsControl, {
    position: POSITIONS.CONTROLS_RIGHT,
    index: 4.2,
    getSettings: () => ({
      color: messageSettings.color,
      strokeColor: messageSettings.strokeColor,
      fontSize: messageSettings.fontSize,
      duration: messageSettings.duration,
      area: messageSettings.area,
      mode: messageSettings.mode,
      opacity: messageSettings.opacity,
    }),
    onChange: (partial: Partial<MessageUserSettings>) => {
      if (partial.color) {
        messageSettings.color = partial.color;
      }
      if (partial.strokeColor) {
        messageSettings.strokeColor = partial.strokeColor;
      }
      if (partial.fontSize) {
        messageSettings.fontSize = partial.fontSize;
      }
      if (typeof partial.duration === 'number') {
        messageSettings.duration = partial.duration;
      }
      if (typeof partial.area === 'number') {
        messageSettings.area = sanitizeMessageArea(partial.area);
      }
      if (partial.mode) {
        messageSettings.mode = partial.mode;
      }
      if (typeof partial.opacity === 'number') {
        messageSettings.opacity = sanitizeMessageOpacity(partial.opacity);
      }
    },
  }) as MessageSettingsControl;

  qualityControlPlugin.value = player.registerPlugin(QualityControl, {
    position: POSITIONS.CONTROLS_RIGHT,
    index: 5,
    options: [...qualityOptions],
    getCurrent: () => currentQuality.value,
    onSelect: async (option: string) => {
      if (option === currentQuality.value) {
        return;
      }
      await switchQuality(option);
    },
  }) as QualityControl;
  qualityControlPlugin.value?.setOptions([...qualityOptions]);
  qualityControlPlugin.value?.updateLabel(currentQuality.value);

  lineControlPlugin.value = player.registerPlugin(LineControl, {
    position: POSITIONS.CONTROLS_RIGHT,
    index: 5.2,
    disable: lineOptionsForPlatform.length === 0,
    options: lineOptionsForPlatform,
    getCurrentKey: () => currentLine.value ?? '',
    getCurrentLabel: () => getCurrentLineLabel(currentLine.value),
    onSelect: async (optionKey: string) => {
      if (optionKey === currentLine.value) {
        return;
      }
      await switchLine(optionKey);
    },
  }) as LineControl;
  lineControlPlugin.value?.setOptions(lineOptionsForPlatform);
  lineControlPlugin.value?.updateLabel(getCurrentLineLabel(currentLine.value));

  arrangeControlClusters(player);

  let overlayInstance = createMessageOverlay(player, messageSettings, isMessageEnabled.value);
  messageInstance.value = overlayInstance;

  player.on('ready', async () => {
    arrangeControlClusters(player);
    ensureMessageOverlayHost(player);
    overlayInstance = overlayInstance ?? createMessageOverlay(player, messageSettings, isMessageEnabled.value);
    messageInstance.value = overlayInstance;
    try {
      if (roomId) {
        await startCurrentMessageListener(platformCode, roomId, overlayInstance);
      }
    } catch (error) {
      console.error('[Player] Failed starting message listener after ready:', error);
    }
    overlayInstance?.play?.();
    updateFullscreenFlag();
  });

  player.on('play', () => {
    overlayInstance?.play?.();
  });

  player.on('pause', () => {
    overlayInstance?.pause?.();
  });

  player.on('destroy', () => {
    overlayInstance?.stop?.();
    overlayInstance = null;
    messageInstance.value = null;
  });

  player.on('error', (error: any) => {
    console.error('[Player] xgplayer error:', error);
    streamError.value = `播放器错误: ${error?.message || error}`;
  });

  player.on('enterFullscreen', () => {
    isInNativePlayerFullscreen.value = true;
    ensureMessageOverlayHost(player);
    overlayInstance = overlayInstance ?? createMessageOverlay(player, messageSettings, isMessageEnabled.value);
    messageInstance.value = overlayInstance;
    overlayInstance?.play?.();
    updateFullscreenFlag();
  });

  player.on('exitFullscreen', () => {
    isInNativePlayerFullscreen.value = false;
    ensureMessageOverlayHost(player);
    overlayInstance = overlayInstance ?? createMessageOverlay(player, messageSettings, isMessageEnabled.value);
    messageInstance.value = overlayInstance;
    updateFullscreenFlag();
  });

  player.on('enterFullscreenWeb', () => {
    isInWebFullscreen.value = true;
    try {
      document.documentElement.classList.add('web-fs-active');
    } catch (error) {
      console.warn('[Player] Failed to set web fullscreen flag:', error);
    }
    ensureMessageOverlayHost(player);
    overlayInstance = overlayInstance ?? createMessageOverlay(player, messageSettings, isMessageEnabled.value);
    messageInstance.value = overlayInstance;
    overlayInstance?.play?.();
    arrangeControlClusters(player);
    updateFullscreenFlag();
  });

  player.on('exitFullscreenWeb', () => {
    isInWebFullscreen.value = false;
    try {
      document.documentElement.classList.remove('web-fs-active');
    } catch (error) {
      console.warn('[Player] Failed to clear web fullscreen flag:', error);
    }
    ensureMessageOverlayHost(player);
    overlayInstance = overlayInstance ?? createMessageOverlay(player, messageSettings, isMessageEnabled.value);
    messageInstance.value = overlayInstance;
    arrangeControlClusters(player);
    updateFullscreenFlag();
  });

  player.on('cssFullscreen_change', (isCssFullscreen: boolean) => {
    isInWebFullscreen.value = isCssFullscreen;
    try {
      if (isCssFullscreen) {
        document.documentElement.classList.add('web-fs-active');
      } else {
        document.documentElement.classList.remove('web-fs-active');
      }
    } catch (error) {
      console.warn('[Player] Failed toggling css fullscreen flag:', error);
    }
    ensureMessageOverlayHost(player);
    overlayInstance = overlayInstance ?? createMessageOverlay(player, messageSettings, isMessageEnabled.value);
    messageInstance.value = overlayInstance;
    if (isCssFullscreen) {
      overlayInstance?.play?.();
    }
    arrangeControlClusters(player);
    updateFullscreenFlag();
  });
}


async function initializePlayerAndStream(
  pRoomId: string, 
  pPlatform: StreamingPlatform,
  _pStreamUrlProp?: string | null, 
  isRefresh: boolean = false,
  oldRoomIdForCleanup?: string | null,
  oldPlatformForCleanup?: StreamingPlatform | null
) {
  isLoadingStream.value = true;
  streamError.value = null;
  isOfflineError.value = false;

  // Detect OS and adjust message font family per platform
  osName.value = await applyMessageFontFamilyForOS();

  if (!isRefresh) {
    messageMessages.value = [];
  }

  if (props.initialError && props.initialError.includes('主播未开播')) {
    streamError.value = props.initialError;
    isOfflineError.value = true;
    playerTitle.value = props.title;
    playerAnchorName.value = props.anchorName;
    playerAvatar.value = props.avatar;
    playerIsLive.value = false;
    destroyPlayerInstance();
    isLoadingStream.value = false;
    return;
  }

  if (oldRoomIdForCleanup && oldPlatformForCleanup !== undefined && oldPlatformForCleanup !== null) {
    await stopCurrentMessageListener(oldPlatformForCleanup);
    if (oldPlatformForCleanup === StreamingPlatform.DOUYU) {
      await stopDouyuProxy();
    }
  } else {
    await stopCurrentMessageListener();
  }

  destroyPlayerInstance();

  const effectiveLine = resolveCurrentLineFor(pPlatform, currentLine.value);

  try {
    let streamConfig: { streamUrl: string; streamType: string | undefined };

    if (pPlatform === StreamingPlatform.DOUYU) {
      if (playerIsLive.value === false) {
        streamError.value = streamError.value || '主播未开播。';
        isOfflineError.value = true;
        isLoadingStream.value = false;
        return;
      }
      streamConfig = await getDouyuStreamConfig(pRoomId, currentQuality.value, effectiveLine);
    } else if (pPlatform === StreamingPlatform.DOUYIN) {
      const douyinConfig = await fetchAndPrepareDouyinStreamConfig(pRoomId, currentQuality.value);
      playerTitle.value = douyinConfig.title;
      playerAnchorName.value = douyinConfig.anchorName;
      playerAvatar.value = douyinConfig.avatar;
      playerIsLive.value = douyinConfig.isLive;

      if (douyinConfig.initialError || !douyinConfig.isLive || !douyinConfig.streamUrl) {
        streamError.value = douyinConfig.initialError || '主播未开播或无法获取直播流。';
        isOfflineError.value = true;
        playerIsLive.value = false;
        isLoadingStream.value = false;
        console.warn(`[Player] Douyin config error or not live: ${streamError.value}`);
        return;
      }

      streamConfig = { streamUrl: douyinConfig.streamUrl, streamType: douyinConfig.streamType };
    } else if (pPlatform === StreamingPlatform.HUYA) {
      streamConfig = await getHuyaStreamConfig(pRoomId, currentQuality.value, effectiveLine);
    } else if (pPlatform === StreamingPlatform.BILIBILI) {
      streamConfig = await getBilibiliStreamConfig(pRoomId, currentQuality.value, props.cookie || undefined);
    } else {
      throw new Error(`不支持的平台: ${pPlatform}`);
    }

    isLoadingStream.value = false;
    await mountXgPlayer(streamConfig.streamUrl, pPlatform, pRoomId, streamConfig.streamType);
  } catch (error: any) {
    console.error(`[Player] Error initializing stream for ${pPlatform} room ${pRoomId}:`, error);
    destroyPlayerInstance();

    const errorMessage = error?.message || '加载直播流失败，请稍后再试。';

    if (errorMessage.includes('主播未开播')) {
      streamError.value = errorMessage;
      isOfflineError.value = true;

      try {
        if (pPlatform === StreamingPlatform.HUYA) {
          const result: any = await invoke('get_huya_unified_cmd', { roomId: pRoomId, quality: currentQuality.value, line: effectiveLine ?? null });
          await ensureProxyStarted();
          playerTitle.value = result?.title ?? props.title;
          playerAnchorName.value = result?.nick ?? props.anchorName;
          playerAvatar.value = proxify((result?.avatar ?? props.avatar ?? '') as string);
        } else if (pPlatform === StreamingPlatform.BILIBILI) {
          const payload = { args: { room_id_str: pRoomId } };
          const savedCookie = (typeof localStorage !== 'undefined') ? (localStorage.getItem('bilibili_cookie') || null) : null;
          const res: any = await invoke('fetch_bilibili_streamer_info', { payload, cookie: savedCookie });
          await ensureProxyStarted();
          playerTitle.value = res?.title ?? props.title;
          playerAnchorName.value = res?.anchor_name ?? props.anchorName;
          playerAvatar.value = proxify((res?.avatar ?? props.avatar ?? '') as string);
        }
      } catch (infoError) {
        console.warn('[Player] Failed to fetch basic streamer info for offline page:', infoError);
      }
    } else {
      streamError.value = errorMessage;
      isOfflineError.value = false;
    }

    isLoadingStream.value = false;
  }
}
const messageManagerContext = {
  messages: messageMessages,
  isMessageEnabled,
  messageSettings: messageSettings,
  isMessageListenerActive,
  unlistenMessageFn,
  props,
};

const startCurrentMessageListener = async (platform: StreamingPlatform, roomId: string, messageOverlay: MessageOverlayInstance | null) => {
  await startMessageListener(messageManagerContext, platform, roomId, messageOverlay);
};

const stopCurrentMessageListener = async (platform?: StreamingPlatform) => {
  await stopMessageListener(messageManagerContext, platform);
};

const retryInitialization = async () => {
  await reloadCurrentStream('refresh');
};

// 画质切换函数
const switchQuality = async (quality: string) => {
  if (isQualitySwitching.value) {
    return;
  }
  if (!qualityOptions.includes(quality as (typeof qualityOptions)[number])) {
    return;
  }
  if (!props.roomId || props.platform == null) {
    emit('request-player-reload');
    return;
  }
  if (quality === currentQuality.value) {
    return;
  }

  isQualitySwitching.value = true;
  const previousQuality = currentQuality.value;

  try {
    currentQuality.value = quality;
    if (typeof window !== 'undefined') {
      window.localStorage.setItem(`${props.platform}_preferred_quality`, quality);
    }
    await reloadCurrentStream('quality');
    console.log(`[Player] 画质切换完成: ${quality}`);
  } catch (error) {
    console.error('[Player] 画质切换失败:', error);
    currentQuality.value = previousQuality;
    if (typeof window !== 'undefined') {
      window.localStorage.setItem(`${props.platform}_preferred_quality`, previousQuality);
    }
  } finally {
    isQualitySwitching.value = false;
  }
};

const switchLine = async (lineKey: string) => {
  if (isLineSwitching.value) {
    return;
  }
  const options = lineOptions.value;
  if (!options.length) {
    return;
  }
  if (!options.some((option) => option.key === lineKey)) {
    return;
  }
  if (!props.roomId || props.platform == null) {
    emit('request-player-reload');
    return;
  }
  if (currentLine.value === lineKey) {
    return;
  }

  isLineSwitching.value = true;
  const previousLine = currentLine.value;

  try {
    currentLine.value = lineKey;
    persistLinePreference(props.platform, lineKey);
    await reloadCurrentStream('line');
    console.log(`[Player] 线路切换完成: ${lineKey}`);
  } catch (error) {
    console.error('[Player] 线路切换失败:', error);
    currentLine.value = previousLine ?? null;
    if (previousLine) {
      persistLinePreference(props.platform, previousLine);
    }
  } finally {
    isLineSwitching.value = false;
  }
};

// 初始化画质偏好
const initializeQualityPreference = () => {
  currentQuality.value = resolveStoredQuality(props.platform);
};

async function reloadCurrentStream(trigger: 'refresh' | 'quality' | 'line' = 'refresh') {
  if (isLoadingStream.value) {
    return;
  }
  if (!props.roomId || props.platform == null) {
    emit('request-player-reload');
    return;
  }
  const isRefreshAction = trigger === 'refresh';
  if (isRefreshAction) {
    isRefreshingStream.value = true;
  }
  try {
    await initializePlayerAndStream(
      props.roomId,
      props.platform,
      props.streamUrl ?? null,
      true,
      props.roomId,
      props.platform,
    );
  } finally {
    if (isRefreshAction) {
      isRefreshingStream.value = false;
    }
  }
  if (trigger === 'quality') {
    qualityControlPlugin.value?.updateLabel(currentQuality.value);
  }
  if (trigger === 'line') {
    lineControlPlugin.value?.updateLabel(getCurrentLineLabel(currentLine.value));
  }
}

const getMessageSettingsSnapshot = (): MessageUserSettings => ({
  color: messageSettings.color,
  strokeColor: messageSettings.strokeColor,
  fontSize: messageSettings.fontSize,
  duration: messageSettings.duration,
  area: sanitizeMessageArea(messageSettings.area),
  mode: messageSettings.mode,
  opacity: sanitizeMessageOpacity(messageSettings.opacity),
});

const persistCurrentMessagePreferences = () => {
  persistMessagePreferences({
    enabled: isMessageEnabled.value,
    settings: getMessageSettingsSnapshot(),
  });
};

registerPlayerWatchers({
  refreshControlPlugin,
  isRefreshingStream,
  qualityControlPlugin,
  isQualitySwitching,
  lineControlPlugin,
  isLineSwitching,
  lineOptions,
  currentLine,
  getLineLabel: getCurrentLineLabel,
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
  playerRoot: () => playerInstance.value?.root as HTMLElement | null,
});

onMounted(async () => {
  // 初始化画质偏好
  initializeQualityPreference();
  
  if (!props.roomId || props.platform == null) {
    if (props.initialError) {
      if (props.initialError.includes('主播未开播')) {
          streamError.value = props.initialError;
          isOfflineError.value = true;
      } else {
          streamError.value = props.initialError;
          isOfflineError.value = false; // Ensure it's not marked as offline for other errors
      }
    }
    isLoadingStream.value = false;
  }

  persistCurrentMessagePreferences();
});

onUnmounted(async () => {
  const platformToStop: StreamingPlatform = props.platform;
  await stopCurrentMessageListener(platformToStop);

  if (props.platform === StreamingPlatform.DOUYU) {
    await stopDouyuProxy();
  }

  destroyPlayerInstance();
  messageMessages.value = []; 
});

</script>
