export type StandaloneRightPanel = {
  type: 'activeDecisions';
};

export type RightPanel =
  | StandaloneRightPanel
  | {
      type: 'forumPost';
      postId: string;
    }
  | null;
