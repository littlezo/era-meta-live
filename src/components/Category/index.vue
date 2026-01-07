<template>
  <div class="category-list" :class="{ 'is-expanded': isExpanded }">
    <CategoryList
      :categories="cate1List"
      :selected-cate1-href="selectedCate1Href"
      :selected-cate2-href="selectedCate2Href"
      :is-expanded="isExpanded"
      @select="handleCategorySelect"
      @toggle-expand="toggleExpand"
      @height-changed="handleCategoryHeightChanged"
    />
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, computed, nextTick, onActivated } from 'vue'
import CategoryList from './components/CategoryList.vue'
import type { CategorySelectedEvent } from '../../platforms/common/types'
import type { Category } from '../../platforms/common/platformApiService'

const props = defineProps<{ categoriesData: Category[] }>()

const emit = defineEmits<{
  (e: 'category-selected', category: CategorySelectedEvent): void
  (e: 'expanded-state-changed', isExpanded: boolean): void
  (e: 'category-section-height-settled'): void
}>()


const cate1List = ref<Category[]>([])
const selectedCate1Href = ref<string | null>(null)
const selectedCate2Href = ref<string | null>(null)

const isExpanded = ref(false)

// 选择一级分类
const selectCate1 = (category: Category) => {
  handleCategorySelect(category, 1)
}

onMounted(() => {
  cate1List.value = Array.isArray(props.categoriesData) ? props.categoriesData : []
  if (cate1List.value.length > 0) {
    if (!selectedCate1Href.value) {
      selectCate1(cate1List.value[0])
    }
  }
  nextTick(() => {
    emit('category-section-height-settled')
  })
})

const currentCate2List = computed(() => {
  if (!selectedCate1Href.value) return []
  const selectedCate1 = cate1List.value.find((c1: Category) => c1.href === selectedCate1Href.value)
  return selectedCate1?.subcategories || []
})

// 处理分类选择
const handleCategorySelect = (category: any, level: number) => {
  if (level === 1) {
    // 选择一级分类
    if (selectedCate1Href.value === category.href) return;
    selectedCate1Href.value = category.href;
    selectedCate2Href.value = null;
    
    // 自动选择第一个二级分类
    if (category.subcategories && category.subcategories.length > 0) {
      handleCate2Select(category.subcategories[0]);
    }
    
    if (isExpanded.value) {
      toggleExpand();
    }
  } else if (level === 2) {
    // 选择二级分类
    handleCate2SelectAndCollapse(category);
  }
  
  nextTick(() => {
    emit('category-section-height-settled');
  });
}

const handleCate2Select = (cate2: any) => {
  selectedCate2Href.value = cate2.href;
  const selectedCate1 = cate1List.value.find((c1: any) => c1.href === selectedCate1Href.value);
  if (selectedCate1) {
    emit('category-selected', {
      type: 'cate2',
      cate1Href: selectedCate1.href,
      cate2Href: cate2.href,
      cate1Name: selectedCate1.name,
      cate2Name: cate2.name,
    });
  }
}

const handleCate2SelectAndCollapse = (cate2: any) => {
  handleCate2Select(cate2);
  if (isExpanded.value) {
    toggleExpand();
  }
}

onActivated(() => {
  const currentSelectedCate1 = cate1List.value.find((c1: Category) => c1.href === selectedCate1Href.value);
  const cate2List = currentCate2List.value || [];
  const currentSelectedCate2 = cate2List.find((c2: Category) => c2.href === selectedCate2Href.value);

  if (currentSelectedCate1 && currentSelectedCate2) {
    emit('category-selected', {
      type: 'cate2',
      cate1Href: currentSelectedCate1.href,
      cate2Href: currentSelectedCate2.href,
      cate1Name: currentSelectedCate1.name,
      cate2Name: currentSelectedCate2.name,
    });
  } else if (currentSelectedCate1 && !selectedCate2Href.value) {
    if (cate2List.length > 0) {
      handleCate2SelectAndCollapse(cate2List[0]);
    }
  }
  nextTick(() => {
    emit('category-section-height-settled');
  });
})

const toggleExpand = () => {
  isExpanded.value = !isExpanded.value;
  emit('expanded-state-changed', isExpanded.value);
  nextTick(() => {
    emit('category-section-height-settled');
  });
}

const handleCategoryHeightChanged = () => {
  emit('category-section-height-settled');
}
</script>

<style scoped>
.category-list {
  display: flex;
  flex-direction: column;
  background: transparent;
  color: var(--primary-text);
  max-height: 280px;
  min-height: 160px;
  overflow: hidden;
  transition: max-height 0.3s ease;
  will-change: max-height;
  transform: translateZ(0);
  width: 100%;
  position: relative;
}

.category-list.is-expanded {
  max-height: 500px;
}

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

.loading-spinner {
  width: 20px;
  height: 20px;
  border: 2px solid var(--border-color);
  border-top-color: var(--accent-color);
  border-radius: 50%;
  animation: spin 0.8s linear infinite;
  margin-bottom: 12px;
}

.loading-text {
  font-size: 12px;
  font-weight: 500;
  color: var(--secondary-text);
}

@keyframes spin {
  to { transform: rotate(360deg); }
}
</style>
