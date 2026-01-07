<template>
  <div class="category-list-container">
    <!-- 一级分类 -->
    <div class="cate1-list" v-if="categories.length > 0">
      <div
        v-for="(category, index) in categories"
        :key="`cate1-${index}-${category.href}`"
        class="cate1-item"
        :class="{ 'is-selected': category.href === selectedCate1Href }"
        @click="selectCategory(category, 1)"
      >
        <div class="cate1-title">{{ category.name }}</div>
      </div>
    </div>
    
    <!-- 二级分类网格 -->
    <div
      class="cate2-grid"
      v-if="currentCate2List.length > 0"
      :class="{ 'is-expanded': isExpanded }"
    >
      <div class="cate2-grid-header">
        <h3 class="cate2-grid-title">{{ currentCate1Title }}</h3>
        <button
          class="expand-toggle-btn"
          @click="toggleExpand"
          :title="isExpanded ? '收起' : '展开更多'"
        >
          <svg xmlns="http://www.w3.org/2000/svg" width="1em" height="1em" viewBox="0 0 24 24">
            <path fill="currentColor" d="M16.59 8.59L12 13.17L7.41 8.59L6 10l6 6l6-6z"/>
          </svg>
        </button>
      </div>
      
      <div class="cate2-grid-content">
        <div
          v-for="(subCategory, index) in displayedCate2List"
          :key="`cate2-${index}-${subCategory.href}`"
          class="cate2-item"
          :class="{ 'is-selected': subCategory.href === selectedCate2Href }"
          @click="selectCategory(subCategory, 2)"
        >
          <div class="cate2-name">{{ subCategory.name }}</div>
        </div>
      </div>
      
      <!-- 展开/收起指示器 -->
      <div v-if="currentCate2List.length > DISPLAY_LIMIT" class="expand-indicator">
        <div class="expand-mask" v-if="!isExpanded"></div>
        <button
          class="expand-btn"
          @click="toggleExpand"
        >
          {{ isExpanded ? '收起' : '展开更多' }}
          <svg xmlns="http://www.w3.org/2000/svg" width="0.875em" height="0.875em" viewBox="0 0 24 24">
            <path fill="currentColor" d="M16.59 8.59L12 13.17L7.41 8.59L6 10l6 6l6-6z"/>
          </svg>
        </button>
      </div>
    </div>
    
    <!-- 加载状态 -->
    <div v-else class="loading-state">
      <div class="loading-text">正在加载分类数据...</div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, nextTick } from 'vue'

import type { Category } from '../../../platforms/common/platformApiService'

const props = defineProps<{
  categories: Category[]
  selectedCate1Href: string | null
  selectedCate2Href: string | null
  isExpanded: boolean
}>()

const emit = defineEmits<{
  (e: 'select', category: Category, level: number): void
  (e: 'toggle-expand'): void
  (e: 'height-changed'): void
}>()

// 常量定义
const DISPLAY_LIMIT = 12 // 默认显示的二级分类数量

// 当前选中的一级分类
const currentSelectedCate1 = computed(() => {
  if (!props.selectedCate1Href) return null
  return props.categories.find(cate1 => cate1.href === props.selectedCate1Href) || null
})

// 当前一级分类的标题
const currentCate1Title = computed(() => {
  return currentSelectedCate1.value?.name || ''
})

// 当前二级分类列表
const currentCate2List = computed(() => {
  return currentSelectedCate1.value?.subcategories || []
})

// 显示的二级分类列表（根据展开状态）
const displayedCate2List = computed(() => {
  if (props.isExpanded) {
    return currentCate2List.value
  }
  return currentCate2List.value.slice(0, DISPLAY_LIMIT)
})

// 选择分类
const selectCategory = (category: any, level: number) => {
  emit('select', category, level)
}

// 切换展开/折叠
const toggleExpand = () => {
  emit('toggle-expand')
  nextTick(() => {
    emit('height-changed')
  })
}
</script>

<style scoped>
.category-list-container {
  width: 100%;
  display: flex;
  flex-direction: column;
  background: transparent;
}

/* 一级分类样式 */
.cate1-list {
  display: flex;
  overflow-x: auto;
  gap: 8px;
  padding: 12px 16px 0;
  scrollbar-width: none;
  -ms-overflow-style: none;
}

.cate1-list::-webkit-scrollbar {
  display: none;
}

.cate1-item {
  padding: 6px 16px;
  border-radius: 16px;
  background: var(--background-secondary);
  cursor: pointer;
  transition: all 0.2s ease;
  white-space: nowrap;
  flex-shrink: 0;
}

.cate1-item:hover {
  background: var(--background-tertiary);
  transform: translateY(-1px);
}

.cate1-item.is-selected {
  background: var(--accent-color);
  color: white;
}

.cate1-title {
  font-size: 13px;
  font-weight: 500;
  line-height: 20px;
}

/* 二级分类网格样式 */
.cate2-grid {
  padding: 12px 16px;
  position: relative;
  transition: all 0.3s ease;
}

.cate2-grid-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 12px;
}

.cate2-grid-title {
  font-size: 14px;
  font-weight: 600;
  color: var(--primary-text);
  margin: 0;
}

.expand-toggle-btn {
  display: flex;
  align-items: center;
  gap: 4px;
  background: none;
  border: none;
  color: var(--secondary-text);
  font-size: 12px;
  cursor: pointer;
  padding: 4px 8px;
  border-radius: 4px;
  transition: all 0.2s ease;
}

.expand-toggle-btn:hover {
  background: var(--background-tertiary);
  color: var(--primary-text);
}

.cate2-grid-content {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(80px, 1fr));
  gap: 8px;
}

.cate2-item {
  padding: 8px;
  background: var(--background-secondary);
  border-radius: 8px;
  cursor: pointer;
  transition: all 0.2s ease;
  text-align: center;
}

.cate2-item:hover {
  background: var(--background-tertiary);
  transform: translateY(-1px);
}

.cate2-item.is-selected {
  background: var(--accent-color);
  color: white;
}

.cate2-name {
  font-size: 12px;
  font-weight: 500;
  line-height: 16px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

/* 展开指示器 */
.expand-indicator {
  margin-top: 12px;
  text-align: center;
  position: relative;
}

.expand-mask {
  position: absolute;
  bottom: 32px;
  left: 0;
  right: 0;
  height: 32px;
  background: linear-gradient(to bottom, transparent, var(--background-primary));
  pointer-events: none;
}

.expand-btn {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  background: var(--background-secondary);
  border: 1px solid var(--border-color);
  color: var(--secondary-text);
  font-size: 12px;
  font-weight: 500;
  padding: 6px 12px;
  border-radius: 16px;
  cursor: pointer;
  transition: all 0.2s ease;
}

.expand-btn:hover {
  background: var(--background-tertiary);
  color: var(--primary-text);
  border-color: var(--accent-color);
}

.expand-btn svg {
  transition: transform 0.2s ease;
}

/* 加载状态 */
.loading-state {
  padding: 32px 16px;
  text-align: center;
  color: var(--secondary-text);
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  height: 100%;
}

.loading-text {
  font-size: 12px;
  font-weight: 500;
  color: var(--secondary-text);
}
</style>
