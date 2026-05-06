import { useEffect, useRef } from "react";
import { useNavigate } from "react-router-dom";

import { Path } from "../constant";
import { useChatStore } from "../store";

export function NewChat() {
  const chatStore = useChatStore();
  const navigate = useNavigate();
  const started = useRef(false);

  useEffect(() => {
    if (started.current) return;
    started.current = true;
    chatStore.newSession();
    navigate(Path.Chat);
  }, [chatStore, navigate]);

  return null;
}
