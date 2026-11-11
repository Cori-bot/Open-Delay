import { useState } from 'react';
import { createRoot } from 'react-dom/client';
import { SelectField } from '../src/components/SelectField';
import { Modal } from '../src/components/Modal';
import '../src/studio.css';
import '../src/support.css';

function Fixture() {
  const [value, setValue] = useState('one');
  const [modal, setModal] = useState(false);
  const options = [{value:'one',label:'Premier choix'}, {value:'two',label:'Deuxième choix'}, {value:'three',label:'Troisième choix'}];
  return <><label>Choix<SelectField label="Choix" value={value} options={options} onChange={setValue}/></label><output>{value}</output><SelectField label="Indisponible" value={value} options={options} disabled onChange={setValue}/><button onClick={() => setModal(true)}>Ouvrir dialogue</button>{modal && <Modal title="Configuration" close={() => setModal(false)}><label>Dans le dialogue<SelectField label="Dans le dialogue" value={value} options={options} onChange={setValue}/></label></Modal>}</>;
}
createRoot(document.getElementById('root')!).render(<Fixture/>);
