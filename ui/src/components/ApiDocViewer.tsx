import { useEffect, useRef } from 'react';
import SwaggerUI from 'swagger-ui-react';
import 'swagger-ui-react/swagger-ui.css';

interface ApiDocViewerProps {
  openApiSpec: object;
  operationId: string;
}

export default function ApiDocViewer({
  openApiSpec,
  operationId,
}: ApiDocViewerProps) {
  const containerRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    // Scroll to the selected operation after Swagger UI renders
    const timer = window.setTimeout(() => {
      if (!containerRef.current) return;

      const selector = `[id*="operations"][id*="${operationId}"]`;
      const operationElement = containerRef.current.querySelector(selector);

      if (operationElement) {
        operationElement.scrollIntoView({ behavior: 'smooth', block: 'start' });
      }
    }, 800);

    return () => window.clearTimeout(timer);
  }, [operationId]);

  return (
    <div ref={containerRef} className="swagger-ui-container">
      <SwaggerUI
        spec={openApiSpec}
        docExpansion="full"
        defaultModelsExpandDepth={1}
        defaultModelExpandDepth={1}
        displayRequestDuration={true}
        filter={false}
        showExtensions={true}
        showCommonExtensions={true}
        tryItOutEnabled={true}
        deepLinking={true}
      />
    </div>
  );
}
