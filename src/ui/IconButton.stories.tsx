import { RefreshCw, Save, Trash2 } from 'lucide-react';
import type { Meta, StoryObj } from '@storybook/react-vite';
import { IconButton } from './IconButton';

const meta = {
  title: 'UI/IconButton',
  component: IconButton,
  args: {
    icon: Save,
    label: 'Save',
  },
  argTypes: {
    variant: {
      control: 'select',
      options: ['default', 'quiet', 'danger'],
    },
  },
} satisfies Meta<typeof IconButton>;

export default meta;

type Story = StoryObj<typeof meta>;

export const Default: Story = {};

export const Quiet: Story = {
  args: {
    icon: RefreshCw,
    label: 'Refresh',
    variant: 'quiet',
  },
};

export const Busy: Story = {
  args: {
    busy: true,
    icon: RefreshCw,
    label: 'Refreshing',
    variant: 'quiet',
  },
};

export const Danger: Story = {
  args: {
    icon: Trash2,
    label: 'Delete',
    variant: 'danger',
  },
};
