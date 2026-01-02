import DanmuJs from 'danmu.js';
import type Player from 'xgplayer';

import { sanitizeMessageArea, sanitizeMessageOpacity } from './constants';
import type { MessageOverlayInstance } from './types';
import type { MessageUserSettings } from './constants';

export const ensureMessageOverlayHost = (player: Player): HTMLElement | null => {
  const root = player.root as HTMLElement | undefined;
  if (!root) {
    return null;
  }

  let host = root.querySelector('.player-message-overlay') as HTMLElement | null;
  if (!host) {
    host = document.createElement('div');
    host.className = 'player-message-overlay';
  }

  const videoContainer = root.querySelector('xg-video-container');
  if (videoContainer && host.parentElement !== videoContainer) {
    videoContainer.appendChild(host);
  } else if (!videoContainer && host.parentElement !== root) {
    root.appendChild(host);
  } else if (!host.parentElement) {
    root.appendChild(host);
  }

  return host;
};

export const applyMessageOverlayPreferences = (
  overlay: MessageOverlayInstance | null,
  messageSettings: MessageUserSettings,
  isMessageEnabled: boolean,
  playerRoot?: HTMLElement | null,
) => {
  if (!overlay) {
    return;
  }
  const host = playerRoot?.querySelector('.player-message-overlay') as HTMLElement | null;
  const fontSizeValue = parseInt(messageSettings.fontSize, 10);
  if (!Number.isNaN(fontSizeValue)) {
    try {
      overlay.setFontSize?.(fontSizeValue);
    } catch (error) {
      console.warn('[Player] Failed to apply message font size:', error);
    }
  }
  try {
    const areaValue = sanitizeMessageArea(messageSettings.area);
    overlay.setArea?.({ start: 0, end: areaValue });
  } catch (error) {
    console.warn('[Player] Failed to apply message area:', error);
  }
  try {
    overlay.setAllDuration?.('scroll', messageSettings.duration);
    overlay.setAllDuration?.('top', messageSettings.duration);
    overlay.setAllDuration?.('bottom', messageSettings.duration);
  } catch (error) {
    // Non-critical for players that do not support bulk duration updates
  }
  try {
    const normalizedOpacity = sanitizeMessageOpacity(messageSettings.opacity);
    const nextOpacity = isMessageEnabled ? normalizedOpacity : 0;
    overlay.setOpacity?.(nextOpacity);
    host?.style.setProperty('--message-opacity', String(nextOpacity));
  } catch (error) {
    // Non-critical
  }
  try {
    host?.style.setProperty('--message-stroke-color', messageSettings.strokeColor);
  } catch (error) {
    console.warn('[Player] Failed to apply message stroke color:', error);
  }
};

export const syncMessageEnabledState = (
  overlay: MessageOverlayInstance | null,
  messageSettings: MessageUserSettings,
  isMessageEnabled: boolean,
  playerRoot?: HTMLElement | null,
) => {
  if (!overlay) {
    return;
  }
  const normalizedOpacity = sanitizeMessageOpacity(messageSettings.opacity);
  const targetOpacity = isMessageEnabled ? normalizedOpacity : 0;
  try {
    if (isMessageEnabled) {
      overlay.play?.();
      overlay.show?.('scroll');
      overlay.show?.('top');
      overlay.show?.('bottom');
    } else {
      overlay.pause?.();
    }
    overlay.setOpacity?.(targetOpacity);
    const host = playerRoot?.querySelector('.player-message-overlay') as HTMLElement | null;
    host?.style.setProperty('--message-opacity', String(targetOpacity));
  } catch (error) {
    console.warn('[Player] Failed updating message enabled state:', error);
  }
};

export const createMessageOverlay = (
  player: Player | null,
  messageSettings: MessageUserSettings,
  isMessageEnabled: boolean,
): MessageOverlayInstance | null => {
  if (!player) {
    return null;
  }

  const overlayHost = ensureMessageOverlayHost(player);
  if (!overlayHost) {
    return null;
  }

  overlayHost.innerHTML = '';
  overlayHost.style.setProperty('--message-stroke-color', messageSettings.strokeColor);
  overlayHost.style.setProperty('--message-opacity', String(isMessageEnabled ? sanitizeMessageOpacity(messageSettings.opacity) : 0));

  try {
    const overlay = new DanmuJs({
      container: overlayHost,
      player: player.video || player.media || undefined,
      comments: [],
      mouseControl: false,
      defaultOff: false,
      channelSize: 36,
      containerStyle: {
        pointerEvents: 'none',
      },
    });

    overlay.start?.();
    applyMessageOverlayPreferences(overlay, messageSettings, isMessageEnabled, player.root as HTMLElement);
    syncMessageEnabledState(overlay, messageSettings, isMessageEnabled, player.root as HTMLElement);
    return overlay;
  } catch (error) {
    console.error('[Player] Failed to initialize message overlay:', error);
    return null;
  }
};
