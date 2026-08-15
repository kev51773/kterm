import { getCurrentWindow } from '@tauri-apps/api/window';

export function setupTabBarControls(
  tabBarEl: HTMLElement | null,
  tabsScrollContainer: HTMLElement | null,
  tabsListEl: HTMLElement | null,
  tabScrollLeftBtn: HTMLButtonElement | null,
  tabScrollRightBtn: HTMLButtonElement | null
) {
  function checkTabOverflow() {
    if (!tabsScrollContainer || !tabBarEl) return;
    const isOverflowing = tabsScrollContainer.scrollWidth > tabsScrollContainer.clientWidth + 1;
    tabBarEl.classList.toggle('has-overflow', isOverflowing);

    if (tabScrollLeftBtn && tabScrollRightBtn) {
      tabScrollLeftBtn.disabled = tabsScrollContainer.scrollLeft <= 0;
      tabScrollRightBtn.disabled =
        tabsScrollContainer.scrollLeft + tabsScrollContainer.clientWidth >=
        tabsScrollContainer.scrollWidth - 1;
    }
  }

  if (tabScrollLeftBtn && tabsScrollContainer) {
    tabScrollLeftBtn.addEventListener('click', (e) => {
      e.stopPropagation();
      tabsScrollContainer.scrollBy({ left: -150, behavior: 'smooth' });
    });
  }

  if (tabScrollRightBtn && tabsScrollContainer) {
    tabScrollRightBtn.addEventListener('click', (e) => {
      e.stopPropagation();
      tabsScrollContainer.scrollBy({ left: 150, behavior: 'smooth' });
    });
  }

  if (tabsScrollContainer) {
    tabsScrollContainer.addEventListener('scroll', checkTabOverflow, { passive: true });
  }

  if (typeof ResizeObserver !== 'undefined') {
    const tabResizeObserver = new ResizeObserver(() => {
      checkTabOverflow();
    });
    if (tabsScrollContainer) tabResizeObserver.observe(tabsScrollContainer);
    if (tabsListEl) tabResizeObserver.observe(tabsListEl);
  }

  if (tabBarEl) {
    tabBarEl.addEventListener('dblclick', async (e: MouseEvent) => {
      const target = e.target as HTMLElement;
      if (
        target === tabBarEl ||
        target.id === 'tabs-list' ||
        target.id === 'tabs-scroll-container' ||
        target.id === 'titlebar-drag-spacer'
      ) {
        try {
          await getCurrentWindow().toggleMaximize();
        } catch (err) {
          console.error('Toggle maximize failed:', err);
        }
      }
    });
  }

  return { checkTabOverflow };
}
