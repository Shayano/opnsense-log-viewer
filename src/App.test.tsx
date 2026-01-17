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

  it('displays the component showcase section', () => {
    render(<App />);
    expect(screen.getByRole('heading', { name: /Component Showcase/i })).toBeInTheDocument();
    expect(
      screen.getByText(/Tailwind CSS design system foundation is now configured/i)
    ).toBeInTheDocument();
  });
});
