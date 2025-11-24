import { BrowserRouter, Routes, Route } from 'react-router-dom';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { Layout } from './components/Layout';
import { Dashboard } from './pages/Dashboard';
import { VirtualRouters } from './pages/VirtualRouters';
import { Peers } from './pages/Peers';
import { Dictionaries } from './pages/Dictionaries';

const queryClient = new QueryClient();

function App() {
  return (
    <QueryClientProvider client={queryClient}>
      <BrowserRouter>
        <Routes>
          <Route path="/" element={<Layout />}>
            <Route index element={<Dashboard />} />
            <Route path="vrs" element={<VirtualRouters />} />
            <Route path="peers" element={<Peers />} />
            <Route path="dictionaries" element={<Dictionaries />} />
          </Route>
        </Routes>
      </BrowserRouter>
    </QueryClientProvider>
  );
}

export default App;
