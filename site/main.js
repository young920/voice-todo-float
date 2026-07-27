(function () {
  'use strict';

  // Tab 切换
  document.querySelectorAll('.hero-tab').forEach((tab) => {
    tab.addEventListener('click', () => {
      document.querySelectorAll('.hero-tab').forEach((t) => t.classList.remove('hero-tab-active'));
      tab.classList.add('hero-tab-active');
    });
  });
})();