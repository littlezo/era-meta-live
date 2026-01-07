<template>
  <div class="player-view">
    <MainPlayer v-if="roomId" :platform="platform" :room-id="roomId" :is-followed="isFollowed" @follow="handleFollow" @unfollow="handleUnfollow" @close-player="handleClosePlayer" />
    <div v-else>
      <p>无效的房间ID。</p>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue';
import { useRouter } from 'vue-router';
import MainPlayer from '../components/player/index.vue';
import { useFollowStore } from '../store/followStore';
import type { FollowedStreamer, SupportedPlatform } from '../platforms/common/types';

const props = defineProps<{
  roomId: string;
  platform: SupportedPlatform;
}>();

const router = useRouter();
const followStore = useFollowStore();

const isFollowed = computed(() => {
  return followStore.isFollowed(props.platform, props.roomId);
});

const handleFollow = (streamerData: Omit<FollowedStreamer, 'platform'>) => {
  followStore.followStreamer({ ...streamerData, platform: props.platform, id: props.roomId });
};

const handleUnfollow = (platformId: string) => {
  followStore.unfollowStreamer(props.platform, platformId);
  console.log('PlayerView: Unfollowed', platformId);
};

const handleClosePlayer = () => {
  console.log('PlayerView: Close player event received. Navigating back.');
  router.back();
};

</script>

<style scoped>
.player-view {
  display: flex;
  flex: 1 1 auto;
  flex-direction: column;
  height: 100%;
  min-height: 0;
  width: 100%;
  background-color: transparent;
  align-items: stretch;
}
</style> 
