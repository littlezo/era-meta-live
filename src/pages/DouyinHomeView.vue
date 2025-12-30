<template>
  <div class="douyin-home">
    <div class="douyin-content">
      <div class="left-panel" v-if="categoriesData.length">
        <CommonCategory 
          :categoriesData="categoriesData"
          @category-selected="onCategorySelected" 
        />
      </div>
      <div class="right-panel">
        <CommonStreamerList
          :selectedCategory="selectedCategory"
          :categoriesData="categoriesData"
          platformName="douyin"
          playerRouteName="douyinPlayer"
        />
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import CommonCategory from '../components/CommonCategory/index.vue'
import CommonStreamerList from '../components/CommonStreamerList/index.vue'
import { douyinCategoriesData } from '../platforms/douyin/douyinCategoriesData'
import type { CategorySelectedEvent } from '../platforms/common/categoryTypes'

const categoriesData = douyinCategoriesData
const selectedCategory = ref<CategorySelectedEvent | null>(null)

function onCategorySelected(evt: CategorySelectedEvent) {
  selectedCategory.value = evt
}
</script>

<style scoped>
.douyin-home {
  display: flex;
  flex-direction: column;
  height: 100%;
  background: transparent;
}

.douyin-content {
  display: flex;
  flex-direction: column; /* 改为纵向排列，上下布局 */
  height: 100%;
}

.left-panel {
  width: 100%;
  background: transparent;
  backdrop-filter: none;
  z-index: 10;
  overflow: hidden;
}

.right-panel {
  flex: 1;
  overflow: hidden;
  background: transparent;
}
</style>
