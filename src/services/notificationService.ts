import { useFollowStore } from '../store/followStore';
import { useSettingsStore } from '../store/settingsStore';
import { isPermissionGranted, requestPermission } from '@tauri-apps/plugin-notification';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import type { FollowedStreamer } from '../platforms/common/types';

export class NotificationService {
  private unlisten: (() => void) | null = null;

  constructor() {
    this.initPermission();
  }

  private async initPermission() {
    let permissionGranted = await isPermissionGranted();
    if (!permissionGranted) {
      const permission = await requestPermission();
      permissionGranted = permission === 'granted';
    }
  }

  public async startService() {
    const settingsStore = useSettingsStore();
    const followStore = useFollowStore();
    await this.initPermission();
    await this.stopService();
    const intervalMs = settingsStore.checkInterval * 60 * 1000;
    const streamers = followStore.followedStreamers.map(s => ({
      platform: String(s.platform).toLowerCase(),
      id: s.id,
      nickname: s.nickname,
    }));
    console.log(`[NotificationService] Starting Rust watch service: ${streamers.length} streamers, interval=${settingsStore.checkInterval}min`);
    await invoke('start_follow_watch_service', {
      config: {
        streamers,
        interval_ms: intervalMs,
        enable_notification: settingsStore.enableNotification,
      },
    });
    const un1 = await listen('follow_status_update', (event: any) => {
        const payload = event.payload as {
          platform: string;
          id: string;
          live_status: string;
          nickname?: string;
          room_title?: string;
          avatar_url?: string;
        };
        const platformUpper = String(payload.platform || '').toUpperCase();
        const update: Partial<FollowedStreamer> & { platform: any; id: string } = {
          platform: platformUpper as any,
          id: payload.id,
          liveStatus: payload.live_status as any,
        };
        if (payload.nickname) update.nickname = payload.nickname;
        if (payload.room_title) update.roomTitle = payload.room_title;
        if (payload.avatar_url) update.avatarUrl = payload.avatar_url;
        followStore.updateStreamerDetails(update);
      });
    const un2 = await listen('notify', (event: any) => {
      const p = event.payload as { title: string; body: string };
      // Forward to system notification via plugin
      import('@tauri-apps/plugin-notification').then(mod => {
        mod.sendNotification({ title: p.title, body: p.body });
      });
    });
    this.unlisten = () => {
      try { un1(); } catch {}
      try { un2(); } catch {}
    };
  }
 
  public stopService() {
    if (this.unlisten) {
      try {
        this.unlisten();
      } catch {}
      this.unlisten = null;
    }
    return invoke('stop_follow_watch_service').catch(() => {});
  }
 
  public restartService() {
    this.stopService();
    void this.startService();
  }
 
  public async sendTestNotification() {
    await this.initPermission();
    await invoke('send_test_notification');
  }
}

export const notificationService = new NotificationService();
