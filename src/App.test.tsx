import { describe, it, expect } from 'vitest';
import { render, screen } from './test-utils';
import App from './App';

describe('App', () => {
  it('renders without crashing', () => {
    render(<App />);
    expect(screen.getByText(/Welcome to Tauri \+ React/i)).toBeInTheDocument();
  });

  it('displays the greet button', () => {
    render(<App />);
    expect(screen.getByText(/Greet/i)).toBeInTheDocument();
  });

  it('displays the input field', () => {
    render(<App />);
    const input = screen.getByPlaceholderText(/Enter a name/i);
    expect(input).toBeInTheDocument();
  });
});
