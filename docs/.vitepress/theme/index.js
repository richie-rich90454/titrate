import DefaultTheme from 'vitepress/theme';
import './style.css';
import Breadcrumb from './Breadcrumb.vue';

export default {
  extends: DefaultTheme,
  enhanceApp({ app }) {
    // Register breadcrumb component globally
    app.component('Breadcrumb', Breadcrumb);
  },
};
