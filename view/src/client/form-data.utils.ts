type FormDataValue = string | Blob;

export const createFormData = (
  fields: Record<string, FormDataValue>,
): FormData => {
  const formData = new FormData();
  Object.entries(fields).forEach(([name, value]) => {
    formData.set(name, value);
  });
  return formData;
};

export const getJsonOrFormData = (
  payload: unknown,
  files: Record<string, Blob | undefined>,
): unknown | FormData => {
  const presentFiles = Object.entries(files).filter(
    (entry): entry is [string, Blob] => entry[1] !== undefined,
  );

  if (presentFiles.length === 0) {
    return payload;
  }

  return createFormData({
    payload: JSON.stringify(payload),
    ...Object.fromEntries(presentFiles),
  });
};
