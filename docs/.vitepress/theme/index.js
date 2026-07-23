import DefaultTheme from 'vitepress/theme';
import { defineAsyncComponent } from 'vue';
import './style.css';
import MobileTabBar from './components/MobileTabBar.vue';
import ThemeToggle from './components/ThemeToggle.vue';

// Lazy load components
const MobileDrawer = defineAsyncComponent(() =>
  import('./components/MobileDrawer.vue')
);

const CodePlayground = defineAsyncComponent(() =>
  import('./components/CodePlayground.vue')
);

export default {
  extends: DefaultTheme,
  enhanceApp({ app }) {
    app.component('MobileTabBar', MobileTabBar);
    app.component('MobileDrawer', MobileDrawer);
    app.component('ThemeToggle', ThemeToggle);
    app.component('CodePlayground', CodePlayground);
  },
  setup() {
    if (typeof window !== 'undefined') {
      // Close mobile drawer on route change
      window.addEventListener('hashchange', () => {
        document.body.classList.remove('drawer-open');
      });
    }
  },
};
