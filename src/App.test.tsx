import { describe, it, expect } from 'vitest';
import { render, screen } from './test-utils';
import App from './App';

describe('App', () => {
  it('renders without crashing', () => {
    render(<App />);
    expect(screen.getByText(/OPNsense Log Viewer/i)).toBeInTheDocument();
  });

  it('displays the header with theme toggle', () => {
    render(<App />);
    expect(screen.getByText(/OPNsense Log Viewer/i)).toBeInTheDocument();
    expect(screen.getByLabelText(/Toggle dark mode/i)).toBeInTheDocument();
  });

  it('[Story 0.6] does not display component showcase in production UI', () => {
    render(<App />);
    // Story 0.6: Component Showcase removed - production UI should not show development tools
    expect(screen.queryByRole('heading', { name: /Component Showcase/i })).not.toBeInTheDocument();
    expect(
      screen.queryByText(/Tailwind CSS design system foundation is now configured/i)
    ).not.toBeInTheDocument();
  });
});
