import type { Meta, StoryObj } from '@storybook/react-vite';
import { Button } from './Button';
import { Panel } from './Panel';

const meta = {
  title: 'UI/Panel',
  component: Panel,
  args: {
    children: (
      <>
        <p>Mock/demo content for reviewing spacing and container behavior.</p>
        <p>Nothing in this story represents live product state.</p>
      </>
    ),
    title: 'Review Panel',
  },
} satisfies Meta<typeof Panel>;

export default meta;

type Story = StoryObj<typeof meta>;

export const Basic: Story = {};

export const WithActions: Story = {
  args: {
    actions: (
      <>
        <Button variant="ghost">Cancel</Button>
        <Button variant="primary">Apply</Button>
      </>
    ),
    eyebrow: 'Mock Demo',
    footer: 'Footer metadata can be shown here.',
  },
};
