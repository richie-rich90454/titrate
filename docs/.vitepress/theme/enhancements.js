/**
 * Developer Experience Enhancements for Titrate Documentation
 * Tasks 29-33: Keyboard shortcuts, code copying, search, scroll progress, line numbers
 */

// ===== Task 29: Enhanced Keyboard Shortcuts =====
// VitePress already has '/' for search and 'Esc' to close modals by default
// We add arrow key navigation for pages

function initKeyboardNavigation() {
  const prevLink = document.querySelector('.VPDocFooter .pager-link.prev');
  const nextLink = document.querySelector('.VPDocFooter .pager-link.next');
  
  document.addEventListener('keydown', (e) => {
    // Arrow left/right for page navigation (when not in input field)
    if (e.target.tagName === 'INPUT' || e.target.tagName === 'TEXTAREA') return;
    
    // Left arrow - previous page
    if (e.key === 'ArrowLeft' && prevLink) {
      e.preventDefault();
      prevLink.click();
    }
    
    // Right arrow - next page
    if (e.key === 'ArrowRight' && nextLink) {
      e.preventDefault();
      nextLink.click();
    }
    
    // Arrow up/down for scroll with acceleration
    if (e.key === 'ArrowUp' || e.key === 'ArrowDown') {
      const content = document.querySelector('.VPDoc .content-container');
      if (content) {
        const scrollAmount = e.key === 'ArrowUp' ? -100 : 100;
        content.scrollBy({ top: scrollAmount, behavior: 'smooth' });
      }
    }
    
    // Home key - scroll to top
    if (e.key === 'Home') {
      e.preventDefault();
      window.scrollTo({ top: 0, behavior: 'smooth' });
    }
    
    // End key - scroll to bottom
    if (e.key === 'End') {
      e.preventDefault();
      const docContent = document.querySelector('.VPDoc');
      if (docContent) {
        window.scrollTo({ top: docContent.scrollHeight, behavior: 'smooth' });
      }
    }
  });
}

// ===== Task 30: Enhanced Code Block Copying =====
// Make entire code block clickable, add toast notification, Ctrl+C shortcut

function initCodeBlockEnhancements() {
  // Create toast notification container
  const toastContainer = document.createElement('div');
  toastContainer.className = 'titrate-toast-container';
  toastContainer.setAttribute('aria-live', 'polite');
  document.body.appendChild(toastContainer);
  
  // Toast notification function
  function showToast(message, type = 'success') {
    const toast = document.createElement('div');
    toast.className = `titrate-toast titrate-toast-${type}`;
    toast.textContent = message;
    toast.setAttribute('role', 'status');
    toastContainer.appendChild(toast);
    
    // Animate in
    requestAnimationFrame(() => {
      toast.classList.add('titrate-toast-visible');
    });
    
    // Remove after 2 seconds
    setTimeout(() => {
      toast.classList.remove('titrate-toast-visible');
      setTimeout(() => toast.remove(), 300);
    }, 2000);
  }
  
  // Enhance code blocks
  function enhanceCodeBlocks() {
    const codeBlocks = document.querySelectorAll('.vp-doc div[class*="language-"]');
    
    codeBlocks.forEach((block) => {
      // Skip if already enhanced
      if (block.dataset.enhanced) return;
      block.dataset.enhanced = 'true';
      
      // Make entire block clickable (except for the copy button itself)
      block.addEventListener('click', (e) => {
        // Don't trigger if clicking on existing copy button or language label
        if (e.target.closest('button.copy') || e.target.closest('span.lang')) return;
        
        const code = block.querySelector('code') || block.querySelector('pre');
        if (code) {
          copyToClipboard(code.textContent, block);
        }
      });
      
      // Add cursor pointer style
      block.style.cursor = 'pointer';
      
      // Ctrl+C shortcut when focused
      block.setAttribute('tabindex', '0');
      block.addEventListener('keydown', (e) => {
        if ((e.ctrlKey || e.metaKey) && e.key === 'c') {
          const code = block.querySelector('code') || block.querySelector('pre');
          if (code) {
            e.preventDefault();
            copyToClipboard(code.textContent, block);
          }
        }
      });
      
      // Focus indicator
      block.addEventListener('focus', () => {
        block.classList.add('titrate-code-focused');
      });
      block.addEventListener('blur', () => {
        block.classList.remove('titrate-code-focused');
      });
    });
  }
  
  // Copy to clipboard with fallback
  async function copyToClipboard(text, block) {
    try {
      await navigator.clipboard.writeText(text);
      showToast('Copied to clipboard!', 'success');
      
      // Visual feedback on block
      block.classList.add('titrate-code-copied');
      setTimeout(() => block.classList.remove('titrate-code-copied'), 500);
      
      // Also trigger existing copy button animation if present
      const copyBtn = block.querySelector('button.copy');
      if (copyBtn) {
        copyBtn.classList.add('copied');
        setTimeout(() => copyBtn.classList.remove('copied'), 2000);
      }
    } catch (err) {
      // Fallback for older browsers
      const textarea = document.createElement('textarea');
      textarea.value = text;
      textarea.style.position = 'fixed';
      textarea.style.opacity = '0';
      document.body.appendChild(textarea);
      textarea.select();
      document.execCommand('copy');
      document.body.removeChild(textarea);
      showToast('Copied to clipboard!', 'success');
    }
  }
  
  // Enhance on load and on route change
  enhanceCodeBlocks();
  
  // Re-enhance after route changes
  if (typeof window !== 'undefined') {
    window.addEventListener('hashchange', enhanceCodeBlocks);
    // VitePress uses Vue Router, observe DOM changes
    const observer = new MutationObserver(() => {
      enhanceCodeBlocks();
    });
    observer.observe(document.body, { childList: true, subtree: true });
  }
}

// ===== Task 31: Enhanced Search with Section Filtering =====
// Add section filter tabs, improve relevance, add categories

function initSearchEnhancements() {
  // Section categories based on URL paths
  const sections = {
    guide: { label: 'Guide', paths: ['/guide/'], color: '#e94560' },
    reference: { label: 'Reference', paths: ['/reference/'], color: '#00a8a8' },
    stdlib: { label: 'Stdlib', paths: ['/stdlib/'], color: '#10b981' },
    blog: { label: 'Blog', paths: ['/blog/'], color: '#f59f00' }
  };
  
  // Inject section filter into search modal when it opens
  function injectSectionFilter() {
    const searchModal = document.querySelector('.VPLocalSearchBox > .shell');
    if (!searchModal) return;
    
    // Skip if already has filter
    if (searchModal.querySelector('.titrate-search-filter')) return;
    
    // Create filter container
    const filterContainer = document.createElement('div');
    filterContainer.className = 'titrate-search-filter';
    filterContainer.setAttribute('role', 'tablist');
    filterContainer.setAttribute('aria-label', 'Search section filter');
    
    // Add "All" tab
    const allTab = createFilterTab('all', 'All', null, true);
    filterContainer.appendChild(allTab);
    
    // Add section tabs
    Object.entries(sections).forEach(([key, section]) => {
      const tab = createFilterTab(key, section.label, section.color);
      filterContainer.appendChild(tab);
    });
    
    // Insert after search input wrapper
    const inputWrapper = searchModal.querySelector('.search-input-wrapper');
    if (inputWrapper) {
      inputWrapper.after(filterContainer);
    }
  }
  
  function createFilterTab(key, label, color, active = false) {
    const tab = document.createElement('button');
    tab.className = `titrate-search-filter-tab${active ? ' active' : ''}`;
    tab.dataset.section = key;
    tab.setAttribute('role', 'tab');
    tab.setAttribute('aria-selected', active ? 'true' : 'false');
    tab.textContent = label;
    if (color) {
      tab.style.borderColor = color;
    }
    
    tab.addEventListener('click', () => {
      // Update active state
      const tabs = tab.parentElement.querySelectorAll('.titrate-search-filter-tab');
      tabs.forEach(t => {
        t.classList.remove('active');
        t.setAttribute('aria-selected', 'false');
      });
      tab.classList.add('active');
      tab.setAttribute('aria-selected', 'true');
      
      // Filter search results
      filterSearchResults(key);
    });
    
    return tab;
  }
  
  function filterSearchResults(sectionKey) {
    const results = document.querySelectorAll('.VPLocalSearchBox .search-result');
    
    results.forEach((result) => {
      const link = result.querySelector('a') || result;
      const href = link.getAttribute('href') || '';
      
      if (sectionKey === 'all') {
        result.style.display = '';
        return;
      }
      
      const section = sections[sectionKey];
      if (section && section.paths.some(p => href.includes(p))) {
        result.style.display = '';
      } else {
        result.style.display = 'none';
      }
    });
  }
  
  // Add category badges to search results
  function addCategoryBadges() {
    const results = document.querySelectorAll('.VPLocalSearchBox .search-result');
    
    results.forEach((result) => {
      if (result.dataset.badged) return;
      result.dataset.badged = 'true';
      
      const link = result.querySelector('a') || result;
      const href = link.getAttribute('href') || '';
      
      // Find matching section
      let matchedSection = null;
      Object.entries(sections).forEach(([key, section]) => {
        if (section.paths.some(p => href.includes(p))) {
          matchedSection = { key, ...section };
        }
      });
      
      if (matchedSection) {
        const title = result.querySelector('.title');
        if (title) {
          const badge = document.createElement('span');
          badge.className = `titrate-search-badge titrate-search-badge-${matchedSection.key}`;
          badge.textContent = matchedSection.label;
          badge.style.borderColor = matchedSection.color;
          badge.style.color = matchedSection.color;
          title.prepend(badge);
        }
      }
    });
  }
  
  // Observe search modal
  const observer = new MutationObserver(() => {
    injectSectionFilter();
    addCategoryBadges();
  });
  observer.observe(document.body, { childList: true, subtree: true });
  
  // Initial injection
  injectSectionFilter();
  addCategoryBadges();
}

// ===== Task 32: Scroll Progress Indicator =====
// Progress bar, outline scroll-spy, current section indicator

function initScrollProgress() {
  // Create progress bar
  const progressBar = document.createElement('div');
  progressBar.className = 'titrate-scroll-progress';
  progressBar.setAttribute('role', 'progressbar');
  progressBar.setAttribute('aria-label', 'Reading progress');
  progressBar.setAttribute('aria-valuemin', '0');
  progressBar.setAttribute('aria-valuemax', '100');
  progressBar.setAttribute('aria-valuenow', '0');
  document.body.appendChild(progressBar);
  
  // Create current section indicator
  const sectionIndicator = document.createElement('div');
  sectionIndicator.className = 'titrate-section-indicator';
  sectionIndicator.setAttribute('aria-live', 'polite');
  document.body.appendChild(sectionIndicator);
  
  // Update progress on scroll
  function updateProgress() {
    const docContent = document.querySelector('.VPDoc .content-container');
    if (!docContent) return;
    
    const scrollTop = window.scrollY;
    const docHeight = docContent.scrollHeight - window.innerHeight;
    const progress = Math.min(100, Math.max(0, (scrollTop / docHeight) * 100));
    
    progressBar.style.width = `${progress}%`;
    progressBar.setAttribute('aria-valuenow', Math.round(progress));
    
    // Update section indicator
    updateSectionIndicator();
    
    // Update outline scroll-spy
    updateOutlineScrollSpy();
  }
  
  function updateSectionIndicator() {
    const headings = document.querySelectorAll('.VPDoc .content-container h2, .VPDoc .content-container h3');
    let currentSection = null;
    
    headings.forEach((heading) => {
      const rect = heading.getBoundingClientRect();
      if (rect.top <= 150 && rect.bottom > 50) {
        currentSection = heading.textContent.trim();
      }
    });
    
    if (currentSection) {
      sectionIndicator.textContent = currentSection;
      sectionIndicator.classList.add('titrate-section-indicator-visible');
    } else {
      sectionIndicator.classList.remove('titrate-section-indicator-visible');
    }
  }
  
  function updateOutlineScrollSpy() {
    const headings = document.querySelectorAll('.VPDoc .content-container h2, .VPDoc .content-container h3');
    const outlineLinks = document.querySelectorAll('.VPDocAsideOutline .outline-link');
    
    let activeId = null;
    headings.forEach((heading) => {
      const rect = heading.getBoundingClientRect();
      if (rect.top <= 150 && rect.bottom > 50) {
        activeId = heading.id;
      }
    });
    
    outlineLinks.forEach((link) => {
      const href = link.getAttribute('href');
      if (href && href.replace('#', '') === activeId) {
        link.classList.add('active');
      } else {
        link.classList.remove('active');
      }
    });
  }
  
  // Throttled scroll listener
  let ticking = false;
  window.addEventListener('scroll', () => {
    if (!ticking) {
      requestAnimationFrame(() => {
        updateProgress();
        ticking = false;
      });
      ticking = true;
    }
  });
  
  // Initial update
  updateProgress();
}

// ===== Task 33: Code Block Line Numbers Toggle =====
// Toggle button, CSS, localStorage persistence

function initLineNumbersToggle() {
  // Check localStorage for preference
  const savedPreference = localStorage.getItem('titrate-line-numbers');
  const showLineNumbers = savedPreference !== 'false'; // Default to true
  
  // Apply initial state
  if (!showLineNumbers) {
    document.body.classList.add('titrate-line-numbers-hidden');
  }
  
  // Create toggle button container
  const toggleContainer = document.createElement('div');
  toggleContainer.className = 'titrate-line-numbers-toggle-container';
  toggleContainer.setAttribute('role', 'toolbar');
  toggleContainer.setAttribute('aria-label', 'Code display options');
  
  const toggleBtn = document.createElement('button');
  toggleBtn.className = 'titrate-line-numbers-toggle';
  toggleBtn.setAttribute('role', 'button');
  toggleBtn.setAttribute('aria-pressed', showLineNumbers ? 'true' : 'false');
  toggleBtn.setAttribute('aria-label', showLineNumbers ? 'Hide line numbers' : 'Show line numbers');
  
  // Create icon
  const icon = document.createElement('span');
  icon.className = 'titrate-line-numbers-icon';
  icon.innerHTML = showLineNumbers ? 
    '<svg viewBox="0 0 24 24" width="16" height="16" fill="currentColor"><path d="M3 3h18v18H3V3zm2 2v14h14V5H5zm2 2h10v2H7V7zm0 4h10v2H7v-2zm0 4h10v2H7v-2z"/></svg>' :
    '<svg viewBox="0 0 24 24" width="16" height="16" fill="currentColor"><path d="M3 3h18v18H3V3zm2 2v14h14V5H5zm2 2h2v2H7V7zm0 4h2v2H7v-2zm0 4h2v2H7v-2zm4-8h6v2h-6V7zm0 4h6v2h-6v-2zm0 4h6v2h-6v-2z"/></svg>';
  
  const label = document.createElement('span');
  label.className = 'titrate-line-numbers-label';
  label.textContent = 'Line Numbers';
  
  toggleBtn.appendChild(icon);
  toggleBtn.appendChild(label);
  toggleContainer.appendChild(toggleBtn);
  
  // Insert into nav bar
  const navBar = document.querySelector('.VPNavBar .content');
  if (navBar) {
    navBar.appendChild(toggleContainer);
  }
  
  // Toggle function
  function toggleLineNumbers() {
    const isHidden = document.body.classList.toggle('titrate-line-numbers-hidden');
    localStorage.setItem('titrate-line-numbers', isHidden ? 'false' : 'true');
    
    toggleBtn.setAttribute('aria-pressed', !isHidden ? 'true' : 'false');
    toggleBtn.setAttribute('aria-label', !isHidden ? 'Hide line numbers' : 'Show line numbers');
    
    icon.innerHTML = !isHidden ? 
      '<svg viewBox="0 0 24 24" width="16" height="16" fill="currentColor"><path d="M3 3h18v18H3V3zm2 2v14h14V5H5zm2 2h10v2H7V7zm0 4h10v2H7v-2zm0 4h10v2H7v-2z"/></svg>' :
      '<svg viewBox="0 0 24 24" width="16" height="16" fill="currentColor"><path d="M3 3h18v18H3V3zm2 2v14h14V5H5zm2 2h2v2H7V7zm0 4h2v2H7v-2zm0 4h2v2H7v-2zm4-8h6v2h-6V7zm0 4h6v2h-6v-2zm0 4h6v2h-6v-2z"/></svg>';
  }
  
  toggleBtn.addEventListener('click', toggleLineNumbers);
  
  // Keyboard shortcut: Ctrl+L (or Cmd+L on Mac)
  document.addEventListener('keydown', (e) => {
    if ((e.ctrlKey || e.metaKey) && e.key === 'l') {
      if (e.target.tagName !== 'INPUT' && e.target.tagName !== 'TEXTAREA') {
        e.preventDefault();
        toggleLineNumbers();
      }
    }
  });
}

// ===== Initialize All Enhancements =====
function initAllEnhancements() {
  // Wait for DOM to be ready
  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', () => {
      initKeyboardNavigation();
      initCodeBlockEnhancements();
      initSearchEnhancements();
      initScrollProgress();
      initLineNumbersToggle();
    });
  } else {
    initKeyboardNavigation();
    initCodeBlockEnhancements();
    initSearchEnhancements();
    initScrollProgress();
    initLineNumbersToggle();
  }
  
  // Re-initialize on route change (VitePress SPA navigation)
  if (typeof window !== 'undefined') {
    window.addEventListener('hashchange', () => {
      setTimeout(() => {
        initCodeBlockEnhancements();
        initScrollProgress();
      }, 100);
    });
  }
}

// Export for use in theme/index.js
export default {
  init: initAllEnhancements,
  initKeyboardNavigation,
  initCodeBlockEnhancements,
  initSearchEnhancements,
  initScrollProgress,
  initLineNumbersToggle
};