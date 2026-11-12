import './styles.css';

import './simplified.css';
import './alpha3.css';
import './alpha4.css';
import './integrity.css';
import App from './App.svelte';
import { mount } from 'svelte';

mount(App, { target: document.getElementById('app')! });
