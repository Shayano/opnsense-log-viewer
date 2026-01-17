import { useState } from 'react';
import {
  Button,
  Input,
  Select,
  Checkbox,
  Toggle,
  Modal,
  ProgressBar,
  toast,
} from '../components/base';

export function ComponentShowcase() {
  const [inputValue, setInputValue] = useState('');
  const [inputError, setInputError] = useState('');
  const [selectValue, setSelectValue] = useState('');
  const [checkboxChecked, setCheckboxChecked] = useState(false);
  const [toggleChecked, setToggleChecked] = useState(false);
  const [isModalOpen, setIsModalOpen] = useState(false);
  const [progress, setProgress] = useState(45);

  return (
    <div className="space-y-8">
      <section>
        <h3 className="text-lg font-semibold mb-4">Buttons</h3>
        <div className="flex flex-wrap gap-4">
          <Button variant="primary" size="sm">
            Primary Small
          </Button>
          <Button variant="primary" size="md">
            Primary Medium
          </Button>
          <Button variant="primary" size="lg">
            Primary Large
          </Button>
          <Button variant="secondary" size="md">
            Secondary
          </Button>
          <Button variant="ghost" size="md">
            Ghost
          </Button>
          <Button variant="primary" size="md" disabled>
            Disabled
          </Button>
        </div>
      </section>

      <section>
        <h3 className="text-lg font-semibold mb-4">Inputs</h3>
        <div className="space-y-4 max-w-md">
          <Input
            label="Text Input"
            placeholder="Enter text..."
            value={inputValue}
            onChange={(e) => {
              setInputValue(e.target.value);
              setInputError('');
            }}
          />
          <Input label="Number Input" type="number" placeholder="Enter number..." />
          <Input
            label="Input with Error"
            placeholder="This has an error"
            error="This field is required"
            value={inputError}
            onChange={(e) => setInputError(e.target.value)}
          />
          <Input label="Disabled Input" placeholder="Cannot edit" disabled />
        </div>
      </section>

      <section>
        <h3 className="text-lg font-semibold mb-4">Select</h3>
        <div className="max-w-md">
          <Select
            label="Choose an option"
            placeholder="Select..."
            value={selectValue}
            onChange={setSelectValue}
            options={[
              { value: 'option1', label: 'Option 1' },
              { value: 'option2', label: 'Option 2' },
              { value: 'option3', label: 'Option 3' },
            ]}
          />
        </div>
      </section>

      <section>
        <h3 className="text-lg font-semibold mb-4">Checkbox</h3>
        <div className="space-y-2">
          <Checkbox
            label="Accept terms and conditions"
            checked={checkboxChecked}
            onChange={(e) => setCheckboxChecked(e.target.checked)}
          />
          <Checkbox label="Subscribe to newsletter" checked={false} onChange={() => {}} />
          <Checkbox label="Disabled checkbox" checked={true} disabled onChange={() => {}} />
        </div>
      </section>

      <section>
        <h3 className="text-lg font-semibold mb-4">Toggle</h3>
        <div className="space-y-2">
          <Toggle
            label="Enable notifications"
            checked={toggleChecked}
            onChange={(e) => setToggleChecked(e.target.checked)}
          />
          <Toggle label="Dark mode (example)" checked={true} onChange={() => {}} />
          <Toggle label="Disabled toggle" checked={false} disabled onChange={() => {}} />
        </div>
      </section>

      <section>
        <h3 className="text-lg font-semibold mb-4">Modal</h3>
        <div className="space-y-2">
          <Button onClick={() => setIsModalOpen(true)}>Open Modal</Button>
          <Modal
            isOpen={isModalOpen}
            onClose={() => setIsModalOpen(false)}
            title="Example Modal"
            size="md"
          >
            <p className="text-gray-600 dark:text-gray-400">
              This is a modal dialog with a backdrop and close functionality. Press Escape or click
              outside to close.
            </p>
            <div className="mt-4 flex justify-end gap-2">
              <Button variant="ghost" onClick={() => setIsModalOpen(false)}>
                Cancel
              </Button>
              <Button variant="primary" onClick={() => setIsModalOpen(false)}>
                Confirm
              </Button>
            </div>
          </Modal>
        </div>
      </section>

      <section>
        <h3 className="text-lg font-semibold mb-4">Progress Bar</h3>
        <div className="space-y-4 max-w-md">
          <ProgressBar value={progress} label="Indexing logs" showPercentage />
          <div className="flex gap-2">
            <Button size="sm" onClick={() => setProgress(Math.max(0, progress - 10))}>
              -10%
            </Button>
            <Button size="sm" onClick={() => setProgress(Math.min(100, progress + 10))}>
              +10%
            </Button>
          </div>
          <ProgressBar value={75} label="Upload progress" showPercentage />
          <ProgressBar value={100} showPercentage />
        </div>
      </section>

      <section>
        <h3 className="text-lg font-semibold mb-4">Toast Notifications</h3>
        <div className="flex flex-wrap gap-2">
          <Button onClick={() => toast.success('Operation completed successfully!')}>
            Success Toast
          </Button>
          <Button onClick={() => toast.error('An error occurred!')}>Error Toast</Button>
          <Button onClick={() => toast('This is an info message')}>Info Toast</Button>
        </div>
      </section>
    </div>
  );
}
