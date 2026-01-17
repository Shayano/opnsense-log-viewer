import { ReactElement } from 'react';
import { render, RenderOptions } from '@testing-library/react';

// Add custom render function with providers here
function customRender(
  ui: ReactElement,
  options?: Omit<RenderOptions, 'wrapper'>
) {
  return render(ui, { ...options });
}

export * from '@testing-library/react';
export { customRender as render };
