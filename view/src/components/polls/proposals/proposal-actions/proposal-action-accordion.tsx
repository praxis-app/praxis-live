import {
  Accordion,
  AccordionContent,
  AccordionItem,
  AccordionTrigger,
} from '@/components/ui/accordion';
import { type ReactNode } from 'react';

interface Props {
  children: ReactNode;
  summary: ReactNode;
  value: string;
}

export const ProposalActionAccordion = ({
  children,
  summary,
  value,
}: Props) => (
  <Accordion
    type="single"
    collapsible
    className="mb-2.5 rounded-lg bg-black/2 px-4 dark:bg-black/10"
  >
    <AccordionItem value={value}>
      <AccordionTrigger className="cursor-pointer justify-start gap-2 py-4 text-base hover:no-underline [&>svg]:order-first [&>svg]:size-5">
        <div className="min-w-0 truncate">{summary}</div>
      </AccordionTrigger>
      <AccordionContent className="grid gap-x-6 gap-y-4 px-2 pb-5 sm:grid-cols-2">
        {children}
      </AccordionContent>
    </AccordionItem>
  </Accordion>
);
