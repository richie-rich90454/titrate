/**
 * Developer Experience Enhancements for Titrate Documentation
 * Simplified version - keyboard shortcuts and code copy only
 */

function initKeyboardNavigation() {
  document.addEventListener('keydown', (e) => {
    if (e.target.tagName === 'INPUT' || e.target.tagName === 'TEXTAREA') return;

    // Left arrow - previous page
    if (e.key === 'ArrowLeft') {
      const prevLink = document.querySelector('.VPDocFooter .pager-link.prev');
      if (prevLink) {
        e.preventDefault();
        prevLink.click();
      }
    }

    // Right arrow - next page
    if (e.key === 'ArrowRight') {
      const nextLink = document.querySelector('.VPDocFooter .pager-link.next');
      if (nextLink) {
        e.preventDefault();
        nextLink.click();
      }
    }
  });
}

function initCodeBlockCopy() {
  function enhanceCodeBlocks() {
    document.querySelectorAll('.vp-doc div[class*="language-"]').forEach((block) => {
      if (block.dataset.enhanced) return;
      block.dataset.enhanced = 'true';

      block.addEventListener('click', (e) => {
        if (e.target.closest('button.copy') || e.target.closest('span.lang')) return;

        const code = block.querySelector('code') || block.querySelector('pre');
        if (code) {
          navigator.clipboard.writeText(code.textContent).then(() => {
            const btn = block.querySelector('button.copy');
            if (btn) {
              btn.classList.add('copied');
              setTimeout(() => btn.classList.remove('copied'), 2000);
            }
          });
        }
      });
    });
  }

  enhanceCodeBlocks();
  if (typeof window !== 'undefined') {
    window.addEventListener('hashchange', enhanceCodeBlocks);
  }
}

function initAllEnhancements() {
  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', () => {
      initKeyboardNavigation();
      initCodeBlockCopy();
    });
  } else {
    initKeyboardNavigation();
    initCodeBlockCopy();
  }
}

export default {
  init: initAllEnhancements,
};
