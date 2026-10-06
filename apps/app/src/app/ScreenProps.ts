import type { WalletAnalysis } from '../../frontend-contract/types';
import type { ScreenId } from './navigation';

export interface ScreenProps {
  preview: boolean;
  analysis: WalletAnalysis | null;
  onNavigate: (screen: ScreenId) => void;
}
