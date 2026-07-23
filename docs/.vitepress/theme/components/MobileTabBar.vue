<template>
  <nav class="mobile-tab-bar" role="navigation" aria-label="Main navigation">
    <a
      v-for="tab in tabs"
      :key="tab.label"
      :href="tab.href"
      class="tab-item"
      :class="{ active: isActive(tab.href) }"
      :aria-label="tab.label"
    >
      <svg class="tab-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
        <path :d="tab.icon" stroke-linecap="round" stroke-linejoin="round" />
      </svg>
      <span class="tab-label">{{ tab.label }}</span>
    </a>
  </nav>
</template>

<script setup>
import { computed } from 'vue'
import { useRoute } from 'vitepress'

const route = useRoute()

const tabs = [
  {
    label: 'Guide',
    href: '/guide/getting-started',
    icon: 'M12 6.253v13m0-13C10.832 5.477 9.246 5 7.5 5S4.168 5.477 3 6.253v13C4.168 18.477 5.754 18 7.5 18s3.332.477 4.5 1.253m0-13C13.168 5.477 14.754 5 16.5 5c1.747 0 3.332.477 4.5 1.253v13C19.832 18.477 18.247 18 16.5 18c-1.746 0-3.332.477-4.5 1.253'
  },
  {
    label: 'Stdlib',
    href: '/stdlib/lang',
    icon: 'M20 7l-8-4-8 4m16 0l-8 4m8-4v10l-8 4m0-10L4 7m8 4v10M4 7v10l8 4'
  },
  {
    label: 'Reference',
    href: '/reference/lexer',
    icon: 'M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z'
  },
  {
    label: 'Blog',
    href: '/blog/native-backend-announcement',
    icon: 'M19 20H5a2 2 0 01-2-2V6a2 2 0 012-2h10a2 2 0 012 2v1m2 13a2 2 0 01-2-2V7m2 13a2 2 0 002-2V9a2 2 0 00-2-2h-2m-4-3H9M7 16h6M7 8h6v4H7V8z'
  },
  {
    label: 'More',
    href: '#',
    icon: 'M4 6h16M4 12h16M4 18h16'
  }
]

function isActive(href) {
  if (href === '#') return false
  return route.path.includes(href.split('/').filter(Boolean)[0])
}
</script>

<style scoped>
.mobile-tab-bar {
  display: none;
  position: fixed;
  bottom: 0;
  left: 0;
  right: 0;
  z-index: 100;
  background: var(--tt-bg);
  border-top: 1px solid var(--tt-border);
  padding: 6px 0 env(safe-area-inset-bottom, 6px);
  backdrop-filter: blur(8px);
  background: rgba(255, 255, 255, 0.9);
}

.dark .mobile-tab-bar {
  background: rgba(15, 23, 42, 0.9);
}

@media (max-width: 768px) {
  .mobile-tab-bar {
    display: flex;
    justify-content: space-around;
  }
}

.tab-item {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 2px;
  padding: 4px 8px;
  text-decoration: none;
  color: var(--tt-text-3);
  font-size: 0.65em;
  font-weight: 500;
  border-radius: var(--tt-radius-sm);
  transition: color var(--tt-transition);
  min-width: 48px;
}

.tab-item:active {
  transform: scale(0.95);
}

.tab-item.active {
  color: var(--vp-c-brand-1);
}

.tab-icon {
  width: 22px;
  height: 22px;
}

.tab-label {
  line-height: 1;
}
</style>
