import { ArrowRight, Check } from 'lucide-react';
import type { Meta, StoryObj } from '@storybook/react-vite';
import { Button } from './Button';

const meta = {
  title: 'UI/Button',
  component: Button,
  args: {
    children: 'Save draft',
  },
  argTypes: {
    variant: {
      control: 'select',
      options: ['primary', 'secondary', 'ghost', 'danger'],
    },
  },
} satisfies Meta<typeof Button>;

export default meta;

type Story = StoryObj<typeof meta>;

export const Primary: Story = {
  args: {
    leadingIcon: <Check aria-hidden="true" size={16} />,
    variant: 'primary',
  },
};

export const Busy: Story = {
  args: {
    busy: true,
    children: 'Saving',
    variant: 'primary',
  },
};

export const Disabled: Story = {
  args: {
    children: 'Unavailable',
    disabled: true,
  },
};

export const WithTrailingIcon: Story = {
  args: {
    children: 'Continue',
    trailingIcon: <ArrowRight aria-hidden="true" size={16} />,
    variant: 'secondary',
  },
};
