DELETE FROM index_values;
DELETE FROM user_interests;
DELETE FROM indices;
DELETE FROM interests;
DELETE FROM data_sources;

ALTER SEQUENCE index_values_id_seq RESTART WITH 1;
ALTER SEQUENCE user_interests_id_seq RESTART WITH 1;
ALTER SEQUENCE indices_id_seq RESTART WITH 1;
ALTER SEQUENCE interests_id_seq RESTART WITH 1;
ALTER SEQUENCE data_sources_id_seq RESTART WITH 1;

INSERT INTO interests (name, description)
VALUES
    ('Public Finance', 'Public revenue and expenditure indicators'),
    ('Energy', 'Energy production and processing indicators'),
    ('Skill Development', 'Employment and vocational training indicators'),
    ('Agriculture', 'Agricultural prices and farm-sector indicators'),
    ('Water Resources', 'Water management and watershed development indicators');

INSERT INTO indices (interest_id, name, description)
SELECT interests.id, seeded.name, seeded.description
FROM (
    VALUES
        ('Public Finance', 'Central GST Collections vs. Estimates', 'Year-wise details of actual net central goods and services tax collections compared with budget and revised estimates from 2019-20 to 2023-24'),
        ('Energy', 'Monthly Crude Oil Processed by Refineries', 'Monthly crude oil processed by refineries'),
        ('Skill Development', 'PMKVY Training by State and UT', 'State and UT-wise training data under Pradhan Mantri Kaushal Vikas Yojana from 2018-19 to 2022-23'),
        ('Agriculture', 'Sugarcane Fair and Remunerative Price', 'Year-wise details of hikes in the Fair and Remunerative Price of sugarcane since sugar season 2018-19'),
        ('Water Resources', 'PMKSY Watershed Projects and Funding', 'State and UT-wise projects sanctioned and funds released under the Watershed Development Component of PMKSY 2.0')
) AS seeded(sector_name, name, description)
JOIN interests ON interests.name = seeded.sector_name;

INSERT INTO data_sources (name, url, api_endpoint)
VALUES
    ('Year wise details of actual revenue collection net central goods and services tax (GST) compared to budget/revised estimate from 2019-20 to 2023-24', 'https://data.gov.in/resource/14613c4e-5ab0-4705-b440-e4e49ae345de', 'https://api.data.gov.in/resource/14613c4e-5ab0-4705-b440-e4e49ae345de'),
    ('Monthly Crude Oil Processed by Refineries', 'https://data.gov.in/resource/8d3b6596-b09e-4077-aebf-425193185a5b', 'https://api.data.gov.in/resource/8d3b6596-b09e-4077-aebf-425193185a5b'),
    ('State/UT-wise Training Data under Pradhan Mantri Kaushal Vikas Yojana (PMKVY) from 2018-19 to 2022-23', 'https://data.gov.in/resource/62c17dfe-27ad-466b-9750-89ca736b1211', 'https://api.data.gov.in/resource/62c17dfe-27ad-466b-9750-89ca736b1211'),
    ('Year-wise Details of Hike in the Price of Sugarcane Fair and Remunerative Price (FRP) Sugar Seasons Fixed by the Government since Sugar Season from 2018-19 to 2022-23', 'https://data.gov.in/resource/6546457d-a621-4a61-b114-8b3ad0888142', 'https://api.data.gov.in/resource/6546457d-a621-4a61-b114-8b3ad0888142'),
    ('State/UTs-wise Details of Projects Sanctioned and Funds Released under Watershed Development Component (WDC) of Pradhan Mantri Krishi Sinchayee Yojana (PMKSY) 2.0 as on 30-11-2022', 'https://data.gov.in/resource/370593ce-14d7-4e09-8e14-7327334a8298', 'https://api.data.gov.in/resource/370593ce-14d7-4e09-8e14-7327334a8298');
