import { defineStore } from 'pinia';

interface SettingsState {
  checkInterval: number; // in minutes
  enableNotification: boolean;
  notificationSound: boolean;
}

export const useSettingsStore = defineStore('settings', {
  state: (): SettingsState => ({
    checkInterval: 5,
    enableNotification: true,
    notificationSound: true,
  }),
  actions: {
    loadSettings() {
      const savedSettings = localStorage.getItem('appSettings');
      if (savedSettings) {
        try {
          const parsed = JSON.parse(savedSettings);
          this.checkInterval = parsed.checkInterval ?? 5;
          this.enableNotification = parsed.enableNotification ?? true;
          this.notificationSound = parsed.notificationSound ?? true;
        } catch (e) {
          console.error('Failed to load settings:', e);
        }
      }
    },
    saveSettings() {
      const settingsToSave = {
        checkInterval: this.checkInterval,
        enableNotification: this.enableNotification,
        notificationSound: this.notificationSound,
      };
      localStorage.setItem('appSettings', JSON.stringify(settingsToSave));
    },
    setCheckInterval(interval: number) {
      this.checkInterval = interval;
      this.saveSettings();
    },
    setEnableNotification(enable: boolean) {
      this.enableNotification = enable;
      this.saveSettings();
    },
    setNotificationSound(enable: boolean) {
      this.notificationSound = enable;
      this.saveSettings();
    }
  }
});
