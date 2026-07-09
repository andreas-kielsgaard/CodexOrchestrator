import { useState } from 'react';
import type { Meta, StoryObj } from '@storybook/react-vite';
import { Tabs, type TabItem } from './Tabs';

const demoTabs: TabItem[] = [
  {
    id: 'overview',
    label: 'Overview',
    panel: <p>Mock/demo overview content for checking the selected tab treatment.</p>,
  },
  {
    id: 'details',
    label: 'Details',
    panel: <p>Mock/demo detail content with a little more room for text wrapping.</p>,
  },
  {
    disabled: true,
    id: 'disabled',
    label: 'Disabled',
    panel: <p>This panel is not available in the demo.</p>,
  },
];

function ControlledTabs() {
  const [selectedTabId, setSelectedTabId] = useState(demoTabs[0].id);

  return (
    <Tabs
      ariaLabel="Mock demo sections"
      onChange={setSelectedTabId}
      selectedTabId={selectedTabId}
      tabs={demoTabs}
    />
  );
}

const meta = {
  title: 'UI/Tabs',
  component: Tabs,
  args: {
    ariaLabel: 'Mock demo sections',
    onChange: () => undefined,
    selectedTabId: demoTabs[0].id,
    tabs: demoTabs,
  },
} satisfies Meta<typeof Tabs>;

export default meta;

type Story = StoryObj<typeof meta>;

export const Controlled: Story = {
  render: () => <ControlledTabs />,
};
