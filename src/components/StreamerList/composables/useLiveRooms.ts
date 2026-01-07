import { ref } from 'vue';
import type { Ref } from 'vue';
import type { CommonStreamer, SupportedPlatform } from '../../../platforms/common/types';
import { platformApi, type UnifiedRoomInfo } from '../../../platforms/common/platformApiService';

const PAGE_SIZE = 20;

export function useLiveRooms(
  platform: SupportedPlatform,
  categoryIdRef: Ref<string | null>
) {
  const rooms = ref<CommonStreamer[]>([]);
  const isLoading = ref(false);
  const isLoadingMore = ref(false);
  const hasMore = ref(true);
  const currentPage = ref(0);
  const currentPlatform = ref(platform);
  
  // 映射统一的房间信息到 CommonStreamer 类型
  const mapUnifiedRoomToCommon = (room: UnifiedRoomInfo): CommonStreamer => ({
    room_id: room.room_id || '',
    title: room.title || '',
    nickname: room.streamer_name || '',
    avatar: room.avatar_url || '',
    room_cover: room.cover_url || '',
    viewer_count_str: room.viewer_count ? room.viewer_count.toString() : '0',
    platform: currentPlatform.value,
    isLive: !!room.live_status,
  });
  
  // 获取直播列表
  const fetchRooms = async (pageToFetch: number, isLoadMore: boolean) => {
    const categoryId = categoryIdRef.value;
    if (!categoryId) {
      rooms.value = [];
      hasMore.value = false;
      currentPage.value = 0;
      return;
    }
    
    if (isLoadMore) isLoadingMore.value = true;
    else isLoading.value = true;
    
    try {
      // 使用统一的 API 服务获取直播列表
      const result = await platformApi.fetchLiveList(
        categoryId,
        pageToFetch,
        PAGE_SIZE,
        undefined,
        currentPlatform.value
      );
      
      const newRooms = (result.items || []).map(mapUnifiedRoomToCommon);
      if (pageToFetch === 0) {
        rooms.value = newRooms;
      } else {
        rooms.value = [...rooms.value, ...newRooms];
      }
      
      hasMore.value = result.has_more;
      currentPage.value = pageToFetch;
    } catch (e) {
      console.error(`[useLiveRooms] Failed to fetch rooms for platform ${currentPlatform.value}:`, e);
      if (pageToFetch === 0) {
        rooms.value = [];
      }
      hasMore.value = false;
    } finally {
      if (isLoadMore) {
        isLoadingMore.value = false;
      } else {
        isLoading.value = false;
      }
    }
  };
  
  // 加载初始直播列表
  const loadInitialRooms = async () => {
    rooms.value = [];
    hasMore.value = true;
    currentPage.value = 0;
    await fetchRooms(0, false);
  };
  
  // 加载更多直播列表
  const loadMoreRooms = async () => {
    if (!hasMore.value || isLoading.value || isLoadingMore.value) return;
    await fetchRooms(currentPage.value + 1, true);
  };
  
  // 切换平台
  const switchPlatform = async (newPlatform: SupportedPlatform) => {
    currentPlatform.value = newPlatform;
    await loadInitialRooms();
  };
  
  // 切换分类
  const switchCategory = async (newCategoryId: string) => {
    categoryIdRef.value = newCategoryId;
    await loadInitialRooms();
  };
  
  return {
    rooms,
    isLoading,
    isLoadingMore,
    hasMore,
    currentPlatform,
    loadInitialRooms,
    loadMoreRooms,
    switchPlatform,
    switchCategory,
  };
}
