<template>
  <MainLayout />
</template>

<script setup lang="ts">
import MainLayout from './layout/MainLayout.vue'
import './styles/global.css'
import { onMounted, watch } from 'vue';
import { useSettingsStore } from './store/settingsStore';
import { notificationService } from './services/notificationService';

const settingsStore = useSettingsStore();

onMounted(() => {
  settingsStore.loadSettings();
  notificationService.startService();
});

watch(() => settingsStore.checkInterval, () => {
  notificationService.restartService();
});
</script>