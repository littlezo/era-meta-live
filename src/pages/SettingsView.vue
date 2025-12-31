<template>
  <div class="settings-view">
    <div class="header">
      <h2>偏好设置</h2>
    </div>
    
    <div class="settings-content">
      <div class="setting-item">
        <div class="setting-label">
          <span>开播检测间隔</span>
          <span class="setting-desc">设置后台自动检测主播开播状态的时间间隔</span>
        </div>
        <div class="setting-control">
          <select v-model.number="settingsStore.checkInterval" @change="handleIntervalChange">
            <option :value="1">1 分钟</option>
            <option :value="5">5 分钟</option>
            <option :value="10">10 分钟</option>
            <option :value="30">30 分钟</option>
            <option :value="60">60 分钟</option>
          </select>
        </div>
      </div>

      <div class="setting-item">
        <div class="setting-label">
          <span>开播通知</span>
          <span class="setting-desc">当关注的主播开播时发送系统通知</span>
        </div>
        <div class="setting-control" style="display: flex; gap: 12px; align-items: center;">
          <button class="test-btn" @click="handleTestNotification">测试</button>
          <label class="switch">
            <input type="checkbox" v-model="settingsStore.enableNotification" @change="handleNotificationChange">
            <span class="slider round"></span>
          </label>
        </div>
      </div>

      <div class="setting-item">
        <div class="setting-label">
          <span>通知提示音</span>
          <span class="setting-desc">接收通知时播放提示音</span>
        </div>
        <div class="setting-control">
          <label class="switch">
            <input type="checkbox" v-model="settingsStore.notificationSound" @change="handleSoundChange">
            <span class="slider round"></span>
          </label>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { onMounted } from 'vue';
import { useSettingsStore } from '../store/settingsStore';
import { notificationService } from '../services/notificationService';

const settingsStore = useSettingsStore();

onMounted(() => {
  settingsStore.loadSettings();
});

const handleIntervalChange = () => {
  settingsStore.saveSettings();
};

const handleTestNotification = () => {
  notificationService.sendTestNotification();
};

const handleNotificationChange = () => {
  settingsStore.saveSettings();
};

const handleSoundChange = () => {
  settingsStore.saveSettings();
};
</script>

<style scoped>
.settings-view {
  padding: 24px;
  color: var(--primary-text);
  height: 100%;
  overflow-y: auto;
  box-sizing: border-box;
}

.header {
  margin-bottom: 32px;
  padding-bottom: 16px;
  border-bottom: 1px solid var(--border-color);
}

.header h2 {
  font-size: 24px;
  font-weight: 600;
  margin: 0;
}

.settings-content {
  display: flex;
  flex-direction: column;
  gap: 24px;
}

.setting-item {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 16px;
  background: var(--glass-bg);
  border: 1px solid var(--glass-border);
  border-radius: var(--radius-md);
  backdrop-filter: var(--glass-blur);
}

.setting-label {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.setting-label span:first-child {
  font-size: 16px;
  font-weight: 500;
  color: var(--primary-text);
}

.setting-desc {
  font-size: 12px;
  color: var(--secondary-text);
}

select {
  padding: 8px 12px;
  border-radius: 6px;
  border: 1px solid var(--border-color);
  background: var(--secondary-bg);
  color: var(--primary-text);
  outline: none;
  font-size: 14px;
}

.test-btn {
  padding: 4px 12px;
  border-radius: 6px;
  border: 1px solid var(--border-color);
  background: var(--secondary-bg);
  color: var(--primary-text);
  font-size: 12px;
  cursor: pointer;
  transition: all 0.2s;
}

.test-btn:hover {
  background: var(--tertiary-bg);
  border-color: var(--accent-color);
}

.test-btn:active {
  transform: scale(0.95);
}

/* Toggle Switch */
.switch {
  position: relative;
  display: inline-block;
  width: 50px;
  height: 24px;
}

.switch input {
  opacity: 0;
  width: 0;
  height: 0;
}

.slider {
  position: absolute;
  cursor: pointer;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background-color: var(--tertiary-bg);
  transition: .4s;
}

.slider:before {
  position: absolute;
  content: "";
  height: 16px;
  width: 16px;
  left: 4px;
  bottom: 4px;
  background-color: white;
  transition: .4s;
}

input:checked + .slider {
  background-color: var(--accent-color);
}

input:focus + .slider {
  box-shadow: 0 0 1px var(--accent-color);
}

input:checked + .slider:before {
  transform: translateX(26px);
}

.slider.round {
  border-radius: 34px;
}

.slider.round:before {
  border-radius: 50%;
}
</style>
