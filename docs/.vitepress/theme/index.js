import DefaultTheme from 'vitepress/theme';
import { defineAsyncComponent } from 'vue';
import './style.css';
import Breadcrumb from './Breadcrumb.vue';
import enhancements from './enhancements.js';

// Lazy load CodePlayground component - only loads when actually used on a page
const CodePlayground = defineAsyncComponent(() =>
  import('./components/CodePlayground.vue')
);

export default {
  extends: DefaultTheme,
  enhanceApp({ app }) {
    // Register breadcrumb component globally (used on every page - load synchronously)
    app.component('Breadcrumb', Breadcrumb);
    // Register code playground component globally with lazy loading
    app.component('CodePlayground', CodePlayground);
  },
  setup() {
    // Initialize developer experience enhancements
    // Tasks 29-33: keyboard shortcuts, code copying, search, scroll progress, line numbers
    if (typeof window !== 'undefined') {
      enhancements.init();
    }
  },
};
