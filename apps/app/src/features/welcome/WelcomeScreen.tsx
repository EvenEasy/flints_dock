import {
  Brand,
  MascotHero,
  MechanicalPanel,
  SurfaceFrame,
  TitlePedestal,
} from '../../shared/ui/Design';
import { designAsset } from '../../shared/assets';

/** The welcome composition has one connection action and a neutral disconnected indicator. */
export function WelcomeScreen({ onConnect }: { onConnect: () => void }) {
  return (
    <div className="page page--welcome">
      <Brand />
      <MascotHero />
      <TitlePedestal page="welcome" />
      <MechanicalPanel page="welcome" asset="connect_module_shell" className="connect-module">
        <h2>ПІДКЛЮЧИ ГАМАНЕЦЬ</h2>
        <p className="connect-description">
          Підключи Solana-гаманець,
          <br />
          щоб знайти зайві активи та повернути SOL.
        </p>
        <button className="connect-button" type="button" onClick={onConnect}>
          <SurfaceFrame page="welcome" asset="connect_button_shell" />
          <span>
            <img src={designAsset('welcome', 'wallet_outline_icon')} alt="" aria-hidden="true" />
            CONNECT WALLET
          </span>
        </button>
        <p className="wallet-status">
          <span className="status-led" aria-hidden="true" />
          ГАМАНЕЦЬ НЕ ПІДКЛЮЧЕНО
        </p>
      </MechanicalPanel>
    </div>
  );
}
