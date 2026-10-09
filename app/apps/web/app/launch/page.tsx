import { Head } from '@/components/ui';
import { LaunchForm } from './form';

export default function Launch() {
  return (
    <>
      <Head eyebrow="Launch" title="Launch a token with slots" lede="Four steps. Everything you fix here is permanent: the creator can't change slots, bounds, rules or notice later. Only the items in the slots change, by the rule you pick." />
      <LaunchForm />
    </>
  );
}
