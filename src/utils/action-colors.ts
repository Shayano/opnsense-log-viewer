import { Action } from '@/types/log-entry';
import { CircleX, CircleCheck, CircleAlert, type LucideIcon } from 'lucide-react';

export interface ActionStyle {
  bgClass: string;
  textClass: string;
  icon: LucideIcon;
  iconColor: string;
}

export function getActionStyle(action: Action): ActionStyle {
  switch (action) {
    case Action.BLOCK:
      return {
        bgClass: 'bg-red-100 dark:bg-red-900/20',
        textClass: 'text-red-700 dark:text-red-400',
        icon: CircleX,
        iconColor: 'text-red-600 dark:text-red-500',
      };
    case Action.PASS:
      return {
        bgClass: 'bg-green-100 dark:bg-green-900/20',
        textClass: 'text-green-700 dark:text-green-400',
        icon: CircleCheck,
        iconColor: 'text-green-600 dark:text-green-500',
      };
    case Action.REJECT:
      return {
        bgClass: 'bg-orange-100 dark:bg-orange-900/20',
        textClass: 'text-orange-700 dark:text-orange-400',
        icon: CircleAlert,
        iconColor: 'text-orange-600 dark:text-orange-500',
      };
    default:
      return {
        bgClass: 'bg-gray-100 dark:bg-gray-900/20',
        textClass: 'text-gray-700 dark:text-gray-400',
        icon: CircleAlert,
        iconColor: 'text-gray-600 dark:text-gray-500',
      };
  }
}
