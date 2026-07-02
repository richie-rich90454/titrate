<script setup>
import { computed } from 'vue'
import { useData } from 'vitepress'

const { page, theme } = useData()

// Generate breadcrumb items from the current path
const breadcrumbs = computed(() => {
  const path = page.value.relativePath
  const parts = path.split('/').filter(Boolean)
  
  // Build breadcrumb items
  const items = []
  
  // Determine section based on path
  if (parts[0] === 'guide') {
    items.push({ text: 'Guide', link: '/guide/getting-started' })
  } else if (parts[0] === 'stdlib') {
    items.push({ text: 'Stdlib', link: '/stdlib/lang' })
  } else if (parts[0] === 'reference') {
    items.push({ text: 'Reference', link: '/reference/lexer' })
  } else if (parts[0] === 'blog') {
    items.push({ text: 'Blog', link: '/blog/native-backend-announcement' })
  }
  
  // Add current page (without link)
  const currentTitle = page.value.frontmatter.title || page.value.title
  items.push({ text: currentTitle, link: null, isCurrent: true })
  
  return items
})
</script>

<template>
  <nav
    v-if="breadcrumbs.length > 1"
    class="breadcrumb-container"
    role="navigation"
    aria-label="Breadcrumb"
  >
    <template v-for="(item, index) in breadcrumbs" :key="index">
      <a
        v-if="item.link"
        :href="item.link"
        class="breadcrumb-link"
      >
        {{ item.text }}
      </a>
      <span
        v-else
        class="breadcrumb-current"
        aria-current="page"
      >
        {{ item.text }}
      </span>
      <span
        v-if="index < breadcrumbs.length - 1"
        class="breadcrumb-separator"
      />
    </template>
  </nav>
</template>

<style scoped>
.breadcrumb-container {
  display: flex;
  align-items: center;
  gap: 0.25rem;
  padding: 0.5rem 0 0.75rem 0;
  font-size: 0.85em;
  color: var(--titrate-neutral-5);
  margin-bottom: 0.5rem;
}

.breadcrumb-separator {
  color: var(--titrate-neutral-4);
  padding: 0 0.25rem;
  font-weight: 300;
}

.breadcrumb-separator::before {
  content: '/';
}

.breadcrumb-link {
  color: var(--titrate-neutral-5);
  text-decoration: none;
  transition: color var(--titrate-duration-fast) var(--titrate-ease);
  border-radius: var(--titrate-radius-sm);
  padding: 0.125rem 0.375rem;
}

.breadcrumb-link:hover {
  color: var(--vp-c-brand-1);
  background: var(--vp-c-brand-soft);
  text-decoration: none;
}

.breadcrumb-link:focus-visible {
  outline: 2px solid var(--titrate-accent-blue);
  outline-offset: 2px;
  border-radius: var(--titrate-radius-sm);
}

.breadcrumb-current {
  color: var(--vp-c-brand-1);
  font-weight: 600;
  padding: 0.125rem 0.375rem;
  border-radius: var(--titrate-radius-sm);
  background: var(--vp-c-brand-soft);
}

@media (max-width: 768px) {
  .breadcrumb-container {
    justify-content: center;
    font-size: 0.8em;
    padding: 0.375rem 0 0.5rem 0;
  }

  .breadcrumb-link {
    display: none;
  }

  .breadcrumb-current {
    display: block;
    padding: 0.25rem 0.75rem;
    background: var(--vp-c-brand-soft);
  }

  .breadcrumb-separator {
    display: none;
  }
}
</style>