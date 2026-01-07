<template>
  <div class="home-view-layout">
    <CategoryComponent 
      :categories-data="categoriesData" 
      @category-selected="onCategorySelected"
      class="category-section"
    />
    <StreamerList 
      :selected-category="currentSelectedCategory"
      :categories-data="categoriesData"
      :platform="platform"
      playerRouteName="Player"
      class="streamer-list-section"
    />
  </div>
</template>

<script setup lang="ts">
defineOptions({
  name: 'HomeView'
})

import { ref, watch, onMounted } from 'vue'
import CategoryComponent from '../components/Category/index.vue'
import StreamerList from '../components/StreamerList/index.vue'
import type { CategorySelectedEvent } from '../platforms/common/types'
import { platformApi } from '../platforms/common/platformApiService'
import type { SupportedPlatform } from '../platforms/common/types'
import type { Category as CategoryType } from '../platforms/common/platformApiService'

const props = defineProps<{
  platform: SupportedPlatform
}>()

const categoriesData = ref<CategoryType[]>([])
const currentSelectedCategory = ref<CategorySelectedEvent | null>(null)

// 获取分类数据
const fetchCategoriesData = async () => {
  try {
    // 使用统一的API获取分类数据
    const data = await platformApi.fetchCategories(undefined, props.platform)
    categoriesData.value = data
  } catch (error) {
    console.error('Failed to fetch categories data:', error)
    categoriesData.value = []
  }
}

const onCategorySelected = (categoryEvent: CategorySelectedEvent) => {
  currentSelectedCategory.value = categoryEvent
}

// 监听平台变化，重新获取分类数据
watch(() => props.platform, (newPlatform) => {
  platformApi.setCurrentPlatform(newPlatform)
  fetchCategoriesData()
  currentSelectedCategory.value = null
})

// 组件挂载时获取分类数据
onMounted(() => {
  platformApi.setCurrentPlatform(props.platform)
  fetchCategoriesData()
})
</script>

<style scoped>
.home-view-layout {
  display: flex;
  flex-direction: column;
  height: 100%;
  background: transparent;
  overflow: hidden;
}

.category-section {
  flex-shrink: 0;
  background: transparent;
  backdrop-filter: none;
  z-index: 10;
}

.streamer-list-section {
  flex: 1;
  overflow: hidden;
  background: transparent;
}
</style>