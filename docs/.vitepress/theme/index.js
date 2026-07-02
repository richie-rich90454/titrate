import DefaultTheme from 'vitepress/theme';
import './style.css';
import Breadcrumb from './Breadcrumb.vue';
import CodePlayground from './components/CodePlayground.vue';
import enhancements from './enhancements.js';

export default {
  extends: DefaultTheme,
  enhanceApp({ app }) {
    // Register breadcrumb component globally
    app.component('Breadcrumb', Breadcrumb);
    // Register code playground component globally
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
