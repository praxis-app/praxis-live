import { api } from '@/client/api-client';
import { ChannelSkeleton } from '@/components/channels/channel-skeleton';
import { useQuery } from '@tanstack/react-query';
import { Navigate } from 'react-router-dom';

export const ExplorePage = () => {
  const { data, error, isLoading } = useQuery({
    queryKey: ['servers', 'default'],
    queryFn: api.getDefaultServer,
    refetchOnMount: false,
  });

  if (error) {
    throw error;
  }

  const server = data?.server;

  if (isLoading || !server) {
    return <ChannelSkeleton />;
  }

  return <Navigate to={`/s/${server.slug}`} replace />;
};
